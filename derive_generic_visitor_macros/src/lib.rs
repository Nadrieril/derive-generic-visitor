//! Derive macros for `derive_generic_visitor`.
use proc_macro2::TokenStream;
use syn::*;

pub(crate) use common::*;

mod common;
mod drive;
mod visit;
mod visitable_group;

fn wrap_for_derive(
    input: proc_macro::TokenStream,
    handler: impl Fn(DeriveInput) -> Result<TokenStream>,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    handler(input)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

#[proc_macro_derive(Visitor, attributes(visit))]
pub fn derive_visitor(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    wrap_for_derive(input, |input| visit::impl_visitor(input))
}

#[proc_macro_derive(Visit, attributes(visit))]
pub fn derive_visit(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    wrap_for_derive(input, |input| visit::impl_visit(input, false))
}

#[proc_macro_derive(VisitMut, attributes(visit))]
pub fn derive_visit_mut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    wrap_for_derive(input, |input| visit::impl_visit(input, true))
}

#[proc_macro_derive(Drive, attributes(drive))]
pub fn derive_drive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    wrap_for_derive(input, |input| drive::impl_drive(input, false))
}

#[proc_macro_derive(DriveMut, attributes(drive))]
pub fn derive_drive_mut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    wrap_for_derive(input, |input| drive::impl_drive(input, true))
}

#[proc_macro_attribute]
pub fn visitable_group(
    attrs: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as ItemTrait);
    let attrs = parse_macro_input!(attrs as visitable_group::Options);
    visitable_group::impl_visitable_group(attrs, item)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}
