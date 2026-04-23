use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, Attribute, Ident, ItemImpl, ItemTrait, Result, Token};

use crate::{GenericTy, Names};

enum TyVisitKind {
    Skip,
    Drive,
    Override { skip: bool, name: Ident },
}

struct VisitorDef {
    vis_trait_name: Ident,
    method_name: Ident,
    mutability: Option<Token![mut]>,
    is_two: bool,
    faillible: bool,
    attrs: Vec<Attribute>,
    super_bounds: Vec<syn::TypeParamBound>,
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
        token, Attribute, Ident, Result, Token,
    };

    use crate::{
        visitable_group::{TyVisitKind, VisitorDef},
        NamedGenericTy,
    };

    mod kw {
        syn::custom_keyword!(visitor);
        syn::custom_keyword!(drive);
        syn::custom_keyword!(skip);
        syn::custom_keyword!(infallible);
        syn::custom_keyword!(override_skip);
        syn::custom_keyword!(bounds);
        syn::custom_keyword!(two);
    }

    /// Optional settings that follow the main `visitor(method_name(&[mut|two] TraitName), ...)` args.
    enum VisitorOpt {
        Infallible(#[allow(unused)] kw::infallible),
        Bounds {
            #[allow(unused)]
            kw: kw::bounds,
            #[allow(unused)]
            paren: token::Paren,
            bounds: Punctuated<syn::TypeParamBound, Token![+]>,
        },
    }

    impl Parse for VisitorOpt {
        fn parse(input: ParseStream) -> Result<Self> {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::infallible) {
                Ok(VisitorOpt::Infallible(input.parse()?))
            } else if lookahead.peek(kw::bounds) {
                let content;
                Ok(VisitorOpt::Bounds {
                    kw: input.parse()?,
                    paren: parenthesized!(content in input),
                    bounds: Punctuated::parse_terminated(&content)?,
                })
            } else {
                Err(lookahead.error())
            }
        }
    }

    #[allow(unused)]
    enum VisitableTypeKind {
        Skip(kw::skip),
        Drive(kw::drive),
        Override(Token![override]),
        OverrideSkip(kw::override_skip),
    }

    enum MacroArg {
        /// `visitor(method_name(&[mut|two] trait_name))` sets the name of the visitor trait we will
        /// defer to for visiting.
        SetVisitorTrait {
            #[allow(unused)]
            vis_tok: kw::visitor,
            #[allow(unused)]
            paren: token::Paren,
            method_name: Ident,
            #[allow(unused)]
            paren2: token::Paren,
            attrs: Vec<Attribute>,
            #[allow(unused)]
            ref_tok: Token![&],
            two: Option<kw::two>,
            mutability: Option<Token![mut]>,
            trait_name: Ident,
            opts: Punctuated<VisitorOpt, Token![,]>,
        },
        /// `drive` and `override` set which types are part of the group and whether the visitor
        /// traits are allowed to override the visiting behavior of those types. The syntax is
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
            } else if lookahead.peek(kw::override_skip) {
                MacroArg::SetVisitableTypes {
                    kind: VisitableTypeKind::OverrideSkip(input.parse()?),
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
                let two;
                MacroArg::SetVisitorTrait {
                    vis_tok: input.parse()?,
                    paren: parenthesized!(content in input),
                    method_name: content.parse()?,
                    paren2: parenthesized!(content2 in content),
                    attrs: Attribute::parse_outer(&content2)?,
                    ref_tok: content2.parse()?,
                    two: {
                        two = if content2.peek(kw::two) {
                            Some(content2.parse()?)
                        } else {
                            None
                        };
                        two
                    },
                    mutability: if two.is_some() {
                        None
                    } else {
                        content2.parse()?
                    },
                    trait_name: content2.parse()?,
                    opts: if content.peek(Token![,]) {
                        let _: Token![,] = content.parse()?;
                        Punctuated::parse_terminated(&content)?
                    } else {
                        Punctuated::new()
                    },
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
                        two,
                        attrs,
                        opts,
                        ..
                    } => {
                        let mut faillible = true;
                        let mut super_bounds = vec![];
                        for opt in opts {
                            match opt {
                                VisitorOpt::Infallible(_) => faillible = false,
                                VisitorOpt::Bounds { bounds, .. } => {
                                    super_bounds.extend(bounds);
                                }
                            }
                        }
                        options.visitors.push(VisitorDef {
                            vis_trait_name: trait_name,
                            method_name,
                            mutability,
                            is_two: two.is_some(),
                            faillible,
                            attrs,
                            super_bounds,
                        });
                    }
                    SetVisitableTypes { kind, tys, .. } => {
                        for ty in tys {
                            let kind = match kind {
                                Skip(_) => TyVisitKind::Skip,
                                Drive(_) => TyVisitKind::Drive,
                                Override(_) => TyVisitKind::Override {
                                    skip: false,
                                    name: ty.get_name()?,
                                },
                                OverrideSkip(_) => TyVisitKind::Override {
                                    skip: true,
                                    name: ty.get_name()?,
                                },
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
            let names = if vdef.is_two {
                Names::new_two()
            } else {
                Names::new(vdef.mutability.is_some())
            };
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
            is_two,
            faillible,
            ..
        } = vis_def;
        let return_type = faillible.then_some(quote!(-> #control_flow<V::Break>));
        let other_param = is_two.then(|| quote!(, other: &Self));
        item.items.push(parse_quote!(
            /// Recursively visit this type with the provided visitor. This calls the visitor's `visit_$any`
            /// method if it exists, otherwise `visit_inner`.
            fn #method_name<V: #vis_trait_name>(& #mutability self #other_param, v: &mut V) #return_type;
        ));
    }

    // Implement the visitable trait for the listed types.
    let mut impls: Vec<ItemImpl> = options
        .tys
        .iter()
        .map(|(ty, kind)| {
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
                    is_two,
                    faillible,
                    ..
                } = vis_def;
                let other_param = is_two.then(|| quote!(, other: &Self));
                let other_arg = is_two.then(|| quote!(, other));
                let return_type = faillible.then_some(quote!(-> #control_flow<V::Break>));
                let body = match kind {
                    TyVisitKind::Skip if *faillible => quote!( #control_flow::Continue(()) ),
                    TyVisitKind::Skip => quote!(),
                    TyVisitKind::Drive => quote!(v.visit_inner(self #other_arg)),
                    TyVisitKind::Override { name, .. } => {
                        let method = Ident::new(&format!("visit_{name}"), Span::call_site());
                        quote!( v.#method(self #other_arg) )
                    }
                };
                timpl.items.push(parse_quote!(
                    #[inline]
                    fn #method_name<V: #vis_trait_name>(& #mutability self #other_param, v: &mut V)
                        #return_type
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
    let infallible_wrapper_name =
        Ident::new(&format!("{trait_name}InfallibleWrapper"), Span::call_site());
    let visitor_wrappers = {
        let define_struct = |wrapper_name: &Ident| {
            quote!(
            /// Implementation detail: wrapper that implements `Visit[Mut]<T>` for `T: #trait_name`,
            /// and delegates all the visiting to our trait's `drive[_mut]`. Used in the implementation
            /// of `visit_inner`
            #[repr(transparent)]
            pub struct #wrapper_name<V: ?Sized>(V);
            impl<V: ?Sized> #wrapper_name<V> {
                #[inline]
                fn wrap(x: &mut V) -> &mut Self {
                    // SAFETY: `repr(transparent)`
                    unsafe { std::mem::transmute(x) }
                }
            })
        };
        let wrapper_struct = define_struct(&wrapper_name);
        let wrapper_visitor = quote!(
            #wrapper_struct
            impl<V: Visitor> Visitor for #wrapper_name<V> {
                type Break = V::Break;
            }
        );
        let infallible_wrapper_struct = define_struct(&infallible_wrapper_name);
        let any_infallible_visitor = !visitor_traits.iter().all(|(v, _)| v.faillible);
        let infallible_wrapper_visitor = any_infallible_visitor.then_some(quote!(
            #infallible_wrapper_struct
            impl<V> Visitor for #infallible_wrapper_name<V> {
                type Break = std::convert::Infallible;
            }
        ));
        quote!(
            #wrapper_visitor
            #infallible_wrapper_visitor
        )
    };
    for (vis_def, names) in &visitor_traits {
        let Names { visit_trait, .. } = &names;
        let VisitorDef {
            vis_trait_name,
            mutability,
            is_two,
            faillible,
            ..
        } = vis_def;
        let wrapper_name = if *faillible {
            &wrapper_name
        } else {
            &infallible_wrapper_name
        };

        let y_param = is_two.then(|| quote!(, y: &'s T));
        let y_arg = is_two.then(|| quote!(, y));
        let mut body = quote!(self.0.visit(x #y_arg));
        if !faillible {
            body = quote!(Continue(#body));
        }
        impls.push(parse_quote!(
            impl<'s, V: #vis_trait_name, T: #trait_name> #visit_trait<'s, T> for #wrapper_name<V> {
                #[inline]
                fn visit(&mut self, x: &'s #mutability T #y_param) -> #control_flow<Self::Break> {
                    #body
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
            is_two,
            faillible,
            attrs,
            super_bounds,
        } = vis_def;
        let return_type = faillible.then_some(quote!(-> #control_flow<Self::Break>));
        let return_type_val = if *faillible {
            quote!(-> #control_flow<Self::Break, Self>)
        } else {
            quote!(-> Self)
        };

        // Generate `visit_inner`.
        let y_param_t = is_two.then(|| quote!(, y: &T));
        let y_arg_t_comma = is_two.then(|| quote!(y,));
        let visit_inner = {
            let wrapper_name = if *faillible {
                &wrapper_name
            } else {
                &infallible_wrapper_name
            };
            let mut body = quote! {x.#drive_inner_method(#y_arg_t_comma #wrapper_name::wrap(self))};
            if !*faillible {
                body = quote!(match #body {
                    #control_flow::Continue(x) => x,
                });
            }
            quote! {
                /// Visit the contents of `x`. This calls `self.visit()` on each field of `T`. This
                /// is available for any type whose contents are all `#trait_name`.
                #[inline]
                fn visit_inner<T>(&mut self, x: & #mutability T #y_param_t) #return_type
                where
                    T: #trait_name,
                    T: for<'s> #drive_trait<'s, #wrapper_name<Self>>,
                {
                    #body
                }
            }
        };

        // Visitor trait supertrait constraints.
        let visitor_constraints = if *is_two {
            // VisitTwo requires Break: Default.
            Some(quote!(Visitor<Break: Default>))
        } else {
            faillible.then_some(quote!(Visitor))
        }
        .into_iter()
        .chain(super_bounds.iter().map(|b| quote!(#b)));

        // Generate `visit`, `visit_by_val`, and optionally `visit_by_val_infallible`.
        let y_param_vis = is_two.then(|| quote!(, y: & #mutability T));
        let y_arg_vis = is_two.then(|| quote!(, y));
        let y_arg_vis_comma = is_two.then(|| quote!(y,));
        let visit_method = quote! {
            /// Visit a visitable type. This calls the appropriate method of this trait on `x`
            /// (`visit_$ty` if it exists, `visit_inner` if not).
            #[inline]
            fn visit<'a, T: #trait_name>(&'a mut self, x: & #mutability T #y_param_vis)
                #return_type
            {
                x.#method_name(#y_arg_vis_comma self)
            }
        };
        let visit_by_val_body = if *faillible {
            quote!(self.visit(x #y_arg_vis).map_continue(|()| self))
        } else {
            quote!( self.visit(x); self )
        };
        let visit_by_val_method = quote! {
            /// Convenience alias for method chaining.
            #[inline]
            fn visit_by_val<T: #trait_name>(mut self, x: & #mutability T #y_param_vis)
                #return_type_val
            {
                #visit_by_val_body
            }
        };
        let visit_by_val_infallible = if *faillible && !*is_two {
            Some(quote!(
                /// Convenience when the visitor does not return early.
                #[inline]
                fn visit_by_val_infallible<T: #trait_name>(self, x: & #mutability T) -> Self
                where
                    Self: #the_visitor_trait<Break=::std::convert::Infallible> + Sized,
                {
                    match self.visit_by_val(x) {
                        #control_flow::Continue(x) => x,
                    }
                }
            ))
        } else {
            None
        };

        let mut visitor_trait: ItemTrait = parse_quote! {
            #(#attrs)*
            #vis trait #vis_trait_name: #(#visitor_constraints + )* Sized where  {
                #visit_method
                #visit_by_val_method
                #visit_by_val_infallible
                #visit_inner
            }
        };

        // Add the overrideable methods.
        for (ty, kind) in &options.tys {
            let TyVisitKind::Override { name, skip } = kind else {
                continue;
            };
            let visit_method_name = Ident::new(&format!("visit_{name}"), Span::call_site());
            let enter_method = Ident::new(&format!("enter_{name}"), Span::call_site());
            let exit_method = Ident::new(&format!("exit_{name}"), Span::call_site());
            let (impl_generics, _, where_clause) = ty.generics.split_for_impl();
            let ty = &ty.ty;
            let question_mark = faillible.then_some(quote!(?));
            let return_type = faillible.then_some(quote!(-> #control_flow<Self::Break>));
            let return_value = faillible.then_some(quote!(Continue(())));
            let y_param_ty = is_two.then(|| quote!(, y: &#ty));
            let y_arg = is_two.then(|| quote!(, y));

            let body = (!skip).then_some(quote! {
                self.#enter_method(x #y_arg);
                self.visit_inner(x #y_arg)#question_mark;
                self.#exit_method(x #y_arg);
            });
            visitor_trait.items.push(parse_quote!(
                /// Overrideable method called when visiting a `$ty`. When overriding this method,
                /// call `self.visit_inner(x)` to keep recursively visiting the type, or don't call
                /// it if the contents of `x` should not be visited.
                ///
                /// The default implementation calls `enter_$ty` then `visit_inner` then `exit_$ty`.
                #[inline]
                fn #visit_method_name #impl_generics(&mut self, x: &#mutability #ty #y_param_ty)
                    #return_type
                #where_clause
                {
                    #body
                    #return_value
                }
            ));
            if !skip {
                visitor_trait.items.push(parse_quote!(
                    /// Called when starting to visit a `$ty` (unless `visit_$ty` is overriden).
                    #[inline]
                    fn #enter_method #impl_generics(&mut self, x: &#mutability #ty #y_param_ty)
                        #where_clause {}
                ));
                visitor_trait.items.push(parse_quote!(
                    /// Called when finished visiting a `$ty` (unless `visit_$ty` is overriden).
                    #[inline]
                    fn #exit_method #impl_generics(&mut self, x: &#mutability #ty #y_param_ty)
                        #where_clause {}
                ));
            }
        }
        traits.push(visitor_trait);
    }

    traits.insert(0, item);

    Ok(quote!(
        #visitor_wrappers
        #(#traits)*
        #(#impls)*
    ))
}
