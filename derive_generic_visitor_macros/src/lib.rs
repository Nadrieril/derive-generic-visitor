//! Derive macros for the `Drive`/`DriveMut` traits in `derive_generic_visitor`.
use proc_macro2::TokenStream;
use syn::*;
use token::Mut;

mod drive;
mod visit;

fn expand_with(
    input: proc_macro::TokenStream,
    handler: impl Fn(DeriveInput) -> Result<TokenStream>,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    handler(input)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Shared logic to get the important paths and identifiers for this crate.
struct Names {
    visitor_trait: Path,
    visit_trait: Path,
    drive_trait: Path,
    drive_method: Ident,
    visitor_param: Ident,
    lifetime_param: Lifetime,
    mut_modifier: Option<Mut>,
}
impl Names {
    fn new(mutable: bool) -> Names {
        let crate_path: Path = parse_quote! { ::derive_generic_visitor };
        Names {
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
            drive_method: if mutable {
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

#[proc_macro_derive(Visit, attributes(visit))]
pub fn derive_visit(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |input| visit::impl_visit(input, false))
}

#[proc_macro_derive(VisitMut, attributes(visit))]
pub fn derive_visit_mut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |input| visit::impl_visit(input, true))
}

#[proc_macro_derive(Drive, attributes(drive))]
pub fn derive_drive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |input| drive::impl_drive(input, false))
}

#[proc_macro_derive(DriveMut, attributes(drive))]
pub fn derive_drive_mut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_with(input, |input| drive::impl_drive(input, true))
}
