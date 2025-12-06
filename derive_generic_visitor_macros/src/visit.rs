use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, DeriveInput, GenericParam, Generics, Ident, Result, Type};

use crate::Names;

enum VisitKind {
    /// Visit this type by calling `x.drive_inner(self)?`.
    Drive,
    /// Visit this type by doing nothing.
    Skip,
    /// Visit this type by calling `self.visit_$name(x)?`.
    Override(Ident),
    /// Visit this type by calling `self.enter_$name(x)` then `x.drive_inner(self)?`.
    Enter(Ident),
    /// Visit this type by calling `x.drive_inner(self)?` then `self.exit_$name(x)`.
    Exit(Ident),
}

/// The data of a particular implementation of `Visit[Mut]` we want to generate.
struct Visit {
    generics: Generics,
    ty: Type,
    kind: VisitKind,
}

mod parse {
    use syn::parse::{Parse, ParseStream};
    use syn::punctuated::Punctuated;
    use syn::token::{self};
    use syn::{parenthesized, Attribute, Result, Token};

    use super::{Visit, VisitKind};
    use crate::common::NamedGenericTy;

    mod kw {
        syn::custom_keyword!(skip);
        syn::custom_keyword!(drive);
        syn::custom_keyword!(enter);
        syn::custom_keyword!(exit);
    }

    #[allow(unused)]
    enum VisitKindToken {
        Skip(kw::skip),
        Drive(kw::drive),
        Enter(kw::enter),
        Exit(kw::exit),
        Override(Token![override]),
    }

    #[allow(unused)]
    struct VisitOption {
        /// Optional because `visit(Ty)` is allowed and means the same as `visit(override(Ty))`.
        kind_token: Option<(VisitKindToken, token::Paren)>,
        tys: Punctuated<NamedGenericTy, Token![,]>,
    }

    impl Parse for VisitOption {
        fn parse(input: ParseStream) -> Result<Self> {
            let lookahead = input.lookahead1();
            let visit_kind_token = if lookahead.peek(Token![override]) {
                VisitKindToken::Override(input.parse()?)
            } else if lookahead.peek(kw::enter) {
                VisitKindToken::Enter(input.parse()?)
            } else if lookahead.peek(kw::exit) {
                VisitKindToken::Exit(input.parse()?)
            } else if lookahead.peek(kw::drive) {
                VisitKindToken::Drive(input.parse()?)
            } else if lookahead.peek(kw::skip) {
                VisitKindToken::Skip(input.parse()?)
            } else {
                return match Punctuated::parse_terminated(&input) {
                    Ok(tys) => Ok(VisitOption {
                        kind_token: None,
                        tys,
                    }),
                    Err(_) => Err(lookahead.error()),
                };
            };
            let content;
            Ok(VisitOption {
                kind_token: Some((visit_kind_token, parenthesized!(content in input))),
                tys: Punctuated::parse_terminated(&content)?,
            })
        }
    }

    struct VisitOptions {
        options: Punctuated<VisitOption, Token![,]>,
    }

    impl Parse for VisitOptions {
        fn parse(input: ParseStream) -> Result<Self> {
            Ok(VisitOptions {
                options: Punctuated::parse_terminated(input)?,
            })
        }
    }

    pub fn parse_attrs(attrs: &[Attribute]) -> Result<Vec<super::Visit>> {
        let mut out = Vec::new();
        for attr in attrs {
            if !attr.path().is_ident("visit") {
                continue;
            }
            let visit_options: VisitOptions = attr.parse_args()?;
            for opt in visit_options.options {
                for named_ty in opt.tys {
                    let kind = match &opt.kind_token {
                        Some((tok, _)) => match tok {
                            VisitKindToken::Skip(..) => VisitKind::Skip,
                            VisitKindToken::Drive(..) => VisitKind::Drive,
                            VisitKindToken::Enter(..) => VisitKind::Enter(named_ty.get_name()?),
                            VisitKindToken::Exit(..) => VisitKind::Exit(named_ty.get_name()?),
                            VisitKindToken::Override(..) => {
                                VisitKind::Override(named_ty.get_name()?)
                            }
                        },
                        None => VisitKind::Override(named_ty.get_name()?),
                    };
                    out.push(Visit {
                        kind,
                        ty: named_ty.ty.ty,
                        generics: named_ty.ty.generics,
                    })
                }
            }
        }
        Ok(out)
    }
}

pub fn impl_visit(input: DeriveInput, mutable: bool) -> Result<TokenStream> {
    use VisitKind::*;
    let names = Names::new(mutable);
    let Names {
        visit_trait,
        drive_trait,
        drive_inner_method,
        lifetime_param,
        mut_modifier,
        control_flow,
        ..
    } = &names;

    let visit_options: Vec<Visit> = parse::parse_attrs(&input.attrs)?;

    let name = input.ident;
    let (_, ty_generics, _) = input.generics.split_for_impl();
    let impl_subject = quote! { #name #ty_generics };

    let visit_impls: TokenStream = visit_options
        .iter()
        .map(|visit| {
            let generics = {
                let mut generics = input.generics.clone();
                generics
                    .params
                    .push(GenericParam::Lifetime(parse_quote!(#lifetime_param)));
                generics
                    .params
                    .extend(visit.generics.params.iter().cloned());
                let where_clause = generics.make_where_clause();
                where_clause.predicates.extend(
                    visit
                        .generics
                        .where_clause
                        .iter()
                        .flat_map(|cl| &cl.predicates)
                        .cloned(),
                );
                for param in visit.generics.type_params() {
                    let param = &param.ident;
                    where_clause.predicates.push(parse_quote!(
                        Self: #visit_trait<#lifetime_param, #param>
                    ));
                }
                generics
            };

            let ty = &visit.ty;
            let drive_inner = quote!(
                <#ty as #drive_trait<'_, Self>>::#drive_inner_method(x, self)?;
            );
            let body = match &visit.kind {
                Skip => quote!(),
                Drive => drive_inner,
                Enter(name) => {
                    let method = Ident::new(&format!("enter_{name}"), Span::call_site());
                    quote!( self.#method(x); #drive_inner )
                }
                Exit(name) => {
                    let method = Ident::new(&format!("exit_{name}"), Span::call_site());
                    quote!( #drive_inner self.#method(x); )
                }
                Override(name) => {
                    let method = Ident::new(&format!("visit_{name}"), Span::call_site());
                    quote!( self.#method(x)?; )
                }
            };
            let (impl_generics, _, where_clause) = generics.split_for_impl();
            quote! {
                impl #impl_generics
                    #visit_trait<#lifetime_param, #ty>
                    for #impl_subject
                    #where_clause
                {
                    fn visit(&mut self, x: &#lifetime_param #mut_modifier #ty)
                        -> #control_flow<Self::Break> {
                        #body
                        #control_flow::Continue(())
                    }
                }
            }
        })
        .collect();
    Ok(visit_impls)
}

/// Implement the `Visitor` trait for our type, which provides the `Break` assoc ty.
pub fn impl_visitor(input: DeriveInput) -> Result<TokenStream> {
    let names = Names::new(false);
    let Names { visitor_trait, .. } = &names;

    let name = input.ident;
    let (_, ty_generics, _) = input.generics.split_for_impl();
    let impl_subject = quote! { #name #ty_generics };

    let (impl_generics, _, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #visitor_trait for #impl_subject #where_clause {
            type Break = ::std::convert::Infallible;
        }
    })
}
