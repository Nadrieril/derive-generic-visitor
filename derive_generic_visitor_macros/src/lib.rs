//! Derive macros for the `Drive`/`DriveMut` traits in `derive_generic_visitor`.
use syn::parse_macro_input;
use syn::DeriveInput;

mod drive;

#[proc_macro_derive(Drive, attributes(drive))]
pub fn derive_drive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    drive::impl_drive(input, false)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

#[proc_macro_derive(DriveMut, attributes(drive))]
pub fn derive_drive_mut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    drive::impl_drive(input, true)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}
