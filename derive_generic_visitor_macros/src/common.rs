use convert_case::{Boundary, Case, Casing};
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    token::Mut,
    Error, Generics, Ident, Lifetime, Path, Result, Token, Type,
};

/// Shared logic to get the important paths and identifiers for this crate.
pub struct Names {
    pub control_flow: Path,
    pub visitor_trait: Path,
    pub visit_trait: Path,
    pub drive_trait: Path,
    pub drive_inner_method: Ident,
    pub visitor_param: Ident,
    pub lifetime_param: Lifetime,
    pub mut_modifier: Option<Mut>,
}

impl Names {
    pub fn new(mutable: bool) -> Names {
        let crate_path: Path = parse_quote! { ::derive_generic_visitor };
        Names {
            control_flow: parse_quote!(::std::ops::ControlFlow),
            visitor_trait: parse_quote!( #crate_path::Visitor ),
            visit_trait: if mutable {
                parse_quote!( #crate_path::VisitMut )
            } else {
                parse_quote!( #crate_path::Visit )
            },
            drive_trait: if mutable {
                parse_quote!( #crate_path::DriveMut )
            } else {
                parse_quote!( #crate_path::Drive )
            },
            drive_inner_method: if mutable {
                parse_quote!(drive_inner_mut)
            } else {
                parse_quote!(drive_inner)
            },
            visitor_param: parse_quote!(V),
            lifetime_param: parse_quote!('s),
            mut_modifier: mutable.then(Default::default),
        }
    }
}

/// A type, optionally prefixed with `for<A, B, C: Trait>` generics.
#[derive(Debug)]
pub struct GenericTy {
    pub generics: Generics,
    pub ty: Type,
}

impl Parse for GenericTy {
    fn parse(input: ParseStream) -> Result<Self> {
        let generics = if input.peek(Token![for]) {
            let _: Token![for] = input.parse()?;
            let generics = input.parse()?;
            generics
        } else {
            Generics::default()
        };
        Ok(GenericTy {
            generics,
            ty: input.parse()?,
        })
    }
}

/// A `GenericTy` optionally prefixed with `ident:`
#[derive(Debug)]
pub struct NamedGenericTy {
    pub name: Option<(Ident, Token![:])>,
    pub ty: GenericTy,
}

impl NamedGenericTy {
    pub fn get_name(&self) -> Result<Ident> {
        Ok(match &self.name {
            Some((name, _)) => name.clone(),
            None => match &self.ty.ty {
                Type::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
                    let ident = &path.path.segments[0].ident;
                    let name = ident.to_string();
                    Ident::new(
                        &name
                            .from_case(Case::Pascal)
                            .without_boundaries(&[Boundary::UpperDigit, Boundary::LowerDigit])
                            .to_case(Case::Snake),
                        ident.span(),
                    )
                }
                _ => {
                    return Err(Error::new_spanned(
                        &self.ty.ty,
                        "Cannot make up a method name for this type; \
                        provide one by writing `foo: ` before the type",
                    ))
                }
            },
        })
    }
}

impl Parse for NamedGenericTy {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = if input.peek2(Token![:]) && !input.peek3(Token![:]) {
            Some((input.parse()?, input.parse()?))
        } else {
            None
        };
        Ok(NamedGenericTy {
            name,
            ty: input.parse()?,
        })
    }
}
