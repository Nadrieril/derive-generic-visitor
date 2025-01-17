use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, Ident, ItemImpl, ItemTrait, Result, Token};

use crate::{GenericTy, Names};

enum TyVisitKind {
    Skip,
    Drive,
    Override(Ident),
}

struct VisitorDef {
    vis_trait_name: Ident,
    method_name: Ident,
    mutability: Option<Token![mut]>,
}

#[derive(Default)]
pub struct Options {
    visitors: Vec<VisitorDef>,
    tys: Vec<(GenericTy, TyVisitKind)>,
}

mod parse {
    use syn::{
        parenthesized,
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        token, Ident, Result, Token,
    };

    use crate::{
        visitable_group::{TyVisitKind, VisitorDef},
        NamedGenericTy,
    };

    mod kw {
        syn::custom_keyword!(visitor);
        syn::custom_keyword!(drive);
        syn::custom_keyword!(skip);
    }

    #[allow(unused)]
    enum VisitableTypeKind {
        Skip(kw::skip),
        Drive(kw::drive),
        Override(Token![override]),
    }

    enum MacroArg {
        /// `visitor(method_name(&[mut] trait_name))` sets the name of the visitor trait we will
        /// defer to for visiting.
        SetVisitorTrait {
            #[allow(unused)]
            vis_tok: kw::visitor,
            #[allow(unused)]
            paren: token::Paren,
            method_name: Ident,
            #[allow(unused)]
            paren2: token::Paren,
            #[allow(unused)]
            ref_tok: Token![&],
            mutability: Option<Token![mut]>,
            trait_name: Ident,
        },
        /// `drive` and `override` set which types are part of the group and whether the visitor
        /// traits are allowed to override the visiting behavior of those types. The suntax is
        /// exactly like that of the `Visit[Mut]` traits.
        SetVisitableTypes {
            kind: VisitableTypeKind,
            #[allow(unused)]
            paren: token::Paren,
            tys: Punctuated<NamedGenericTy, Token![,]>,
        },
    }

    impl Parse for MacroArg {
        fn parse(input: ParseStream) -> Result<Self> {
            let lookahead = input.lookahead1();
            let content;
            let content2;
            Ok(if lookahead.peek(Token![override]) {
                MacroArg::SetVisitableTypes {
                    kind: VisitableTypeKind::Override(input.parse()?),
                    paren: parenthesized!(content in input),
                    tys: Punctuated::parse_terminated(&content)?,
                }
            } else if lookahead.peek(kw::drive) {
                MacroArg::SetVisitableTypes {
                    kind: VisitableTypeKind::Drive(input.parse()?),
                    paren: parenthesized!(content in input),
                    tys: Punctuated::parse_terminated(&content)?,
                }
            } else if lookahead.peek(kw::skip) {
                MacroArg::SetVisitableTypes {
                    kind: VisitableTypeKind::Skip(input.parse()?),
                    paren: parenthesized!(content in input),
                    tys: Punctuated::parse_terminated(&content)?,
                }
            } else if lookahead.peek(kw::visitor) {
                MacroArg::SetVisitorTrait {
                    vis_tok: input.parse()?,
                    paren: parenthesized!(content in input),
                    method_name: content.parse()?,
                    paren2: parenthesized!(content2 in content),
                    ref_tok: content2.parse()?,
                    mutability: content2.parse()?,
                    trait_name: content2.parse()?,
                }
            } else {
                return Err(lookahead.error());
            })
        }
    }

    impl Parse for super::Options {
        fn parse(input: ParseStream) -> Result<Self> {
            use MacroArg::*;
            use VisitableTypeKind::*;
            let args: Punctuated<MacroArg, Token![,]> = Punctuated::parse_terminated(input)?;
            let mut options = super::Options::default();
            for arg in args {
                match arg {
                    SetVisitorTrait {
                        trait_name,
                        method_name,
                        mutability,
                        ..
                    } => options.visitors.push(VisitorDef {
                        vis_trait_name: trait_name,
                        method_name,
                        mutability,
                    }),
                    SetVisitableTypes { kind, tys, .. } => {
                        for ty in tys {
                            let kind = match kind {
                                Skip(_) => TyVisitKind::Skip,
                                Drive(_) => TyVisitKind::Drive,
                                Override(_) => TyVisitKind::Override(ty.get_name()?),
                            };
                            options.tys.push((ty.ty, kind));
                        }
                    }
                }
            }
            Ok(options)
        }
    }
}

pub fn impl_visitable_group(options: Options, mut item: ItemTrait) -> Result<TokenStream> {
    let trait_name = &item.ident;
    let shared_names = Names::new(false);
    let control_flow = &shared_names.control_flow;
    let the_visitor_trait = &shared_names.visitor_trait;

    let visitor_traits: Vec<(VisitorDef, Names)> = options
        .visitors
        .into_iter()
        .map(|vdef| {
            let names = Names::new(vdef.mutability.is_some());
            (vdef, names)
        })
        .collect();

    // Add the `drive` methods to the visitable trait, so that visitable types know how to drive
    // the visitor types.
    for (vis_def, _) in &visitor_traits {
        let VisitorDef {
            vis_trait_name,
            method_name,
            mutability,
        } = vis_def;
        item.items.push(parse_quote!(
            /// Recursively visit this type with the provided visitor. This calls the visitor's `visit_$any`
            /// method if it exists, otherwise `visit_inner`.
            fn #method_name<V: #vis_trait_name>(& #mutability self, v: &mut V) -> #control_flow<V::Break>;
        ));
    }

    // Implement the visitable trait for the listed types.
    let mut impls: Vec<ItemImpl> = options
        .tys
        .iter()
        .map(|(ty, kind)| {
            let body = match kind {
                TyVisitKind::Skip => quote!( #control_flow::Continue(()) ),
                TyVisitKind::Drive => quote!(v.visit_inner(self)),
                TyVisitKind::Override(name) => {
                    let method = Ident::new(&format!("visit_{name}"), Span::call_site());
                    quote!( v.#method(self) )
                }
            };
            let (impl_generics, _, where_clause) = ty.generics.split_for_impl();
            let ty = &ty.ty;
            let mut timpl: ItemImpl = parse_quote! {
                impl #impl_generics #trait_name for #ty #where_clause {}
            };
            for (vis_def, _) in &visitor_traits {
                let VisitorDef {
                    vis_trait_name,
                    method_name,
                    mutability,
                } = vis_def;
                timpl.items.push(parse_quote!(
                    fn #method_name<V: #vis_trait_name>(& #mutability self, v: &mut V)
                        -> #control_flow<V::Break>
                    {
                        #body
                    }
                ));
            }
            timpl
        })
        .collect();

    // Define a wrapper type that implements `Visit[Mut]` to pass through the `Drive[Mut]` API.
    let wrapper_name = Ident::new(&format!("{trait_name}Wrapper"), Span::call_site());
    let visitor_wrapper = quote!(
        /// Implementation detail: wrapper that implements `Visit[Mut]<T>` for `T: #trait_name`,
        /// and delegates all the visiting to our trait's `drive[_mut]`. Used in the implementation
        /// of `visit_inner`
        #[repr(transparent)]
        pub struct #wrapper_name<V: ?Sized>(V);
        impl<V: ?Sized> #wrapper_name<V> {
            fn wrap(x: &mut V) -> &mut Self {
                // SAFETY: `repr(transparent)`
                unsafe { std::mem::transmute(x) }
            }
        }
        impl<V: Visitor> Visitor for #wrapper_name<V> {
            type Break = V::Break;
        }
    );
    for (vis_def, names) in &visitor_traits {
        let Names { visit_trait, .. } = &names;
        let VisitorDef {
            vis_trait_name,
            mutability,
            ..
        } = vis_def;
        impls.push(parse_quote!(
            impl<'s, V: #vis_trait_name, T: #trait_name> #visit_trait<'s, T> for #wrapper_name<V> {
                fn visit(&mut self, x: &'s #mutability T) -> #control_flow<Self::Break> {
                    self.0.visit(x)
                }
            }
        ));
    }

    // Define the visitor trait(s).
    let mut traits: Vec<ItemTrait> = vec![];
    let vis = &item.vis;
    for (vis_def, names) in &visitor_traits {
        let Names {
            drive_trait,
            drive_inner_method,
            ..
        } = names;
        let VisitorDef {
            vis_trait_name,
            method_name,
            mutability,
        } = vis_def;
        let mut visitor_trait: ItemTrait = parse_quote! {
            #vis trait #vis_trait_name: Visitor + Sized {
                /// Visit a visitable type. This calls the appropriate method of this trait on `x`
                /// (`visit_$ty` if it exists, `visit_inner` if not).
                fn visit<'a, T: #trait_name>(&'a mut self, x: & #mutability T)
                    -> #control_flow<Self::Break>
                {
                    x.#method_name(self)
                }

                /// Convenience alias for method chaining.
                fn visit_by_val<T: #trait_name>(mut self, x: & #mutability T)
                    -> #control_flow<Self::Break, Self>
                {
                    self.visit(x).map_continue(|()| self)
                }


                /// Convenience when the visitor does not return early.
                fn visit_by_val_infallible<T: #trait_name>(self, x: & #mutability T) -> Self
                where
                    Self: #the_visitor_trait<Break=::std::convert::Infallible> + Sized,
                {
                    match self.visit_by_val(x) {
                        #control_flow::Continue(x) => x,
                    }
                }

                /// Visit the contents of `x`. This calls `self.visit()` on each field of `T`. This
                /// is available for any type whose contents are all `#trait_name`.
                fn visit_inner<T>(&mut self, x: & #mutability T) -> #control_flow<Self::Break>
                where
                   T: for<'s> #drive_trait<'s, #wrapper_name<Self>>,
                {
                    x.#drive_inner_method(#wrapper_name::wrap(self))
                }
            }
        };
        // Add the overrideable methods.
        for (ty, kind) in &options.tys {
            let TyVisitKind::Override(name) = kind else {
                continue;
            };
            let visit_method = Ident::new(&format!("visit_{name}"), Span::call_site());
            let enter_method = Ident::new(&format!("enter_{name}"), Span::call_site());
            let exit_method = Ident::new(&format!("exit_{name}"), Span::call_site());
            let (impl_generics, _, where_clause) = ty.generics.split_for_impl();
            let ty = &ty.ty;
            visitor_trait.items.push(parse_quote!(
                /// Overrideable method called when visiting a `$ty`. When overriding this method,
                /// call `self.visit_inner(x)` to keep recursively visiting the type, or don't call
                /// it if the contents of `x` should not be visited.
                ///
                /// The default implementation calls `enter_$ty` then `visit_inner` then `exit_$ty`.
                fn #visit_method #impl_generics(&mut self, x: &#mutability #ty)
                    -> #control_flow<Self::Break>
                #where_clause
                {
                       self.#enter_method(x);
                       self.visit_inner(x)?;
                       self.#exit_method(x);
                       Continue(())
                }
            ));
            visitor_trait.items.push(parse_quote!(
                /// Called when starting to visit a `$ty` (unless `visit_$ty` is overriden).
                fn #enter_method #impl_generics(&mut self, x: &#mutability #ty) #where_clause {}
            ));
            visitor_trait.items.push(parse_quote!(
                /// Called when finished visiting a `$ty` (unless `visit_$ty` is overriden).
                fn #exit_method #impl_generics(&mut self, x: &#mutability #ty) #where_clause {}
            ));
        }
        traits.push(visitor_trait);
    }

    traits.insert(0, item);

    Ok(quote!(
        #visitor_wrapper
        #(#traits)*
        #(#impls)*
    ))
}
