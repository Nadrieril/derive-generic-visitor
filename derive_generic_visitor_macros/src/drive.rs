use darling::ast::{Data, Fields};
use darling::{FromDeriveInput, FromField, FromVariant};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse_quote, DeriveInput, GenericParam, Generics, Ident, Index, Path, Result, Type};

use crate::Names;

#[derive(FromDeriveInput)]
#[darling(attributes(drive))]
struct MyTypeDecl {
    ident: Ident,
    generics: Generics,
    data: Data<MyVariant, MyField>,
    skip: Option<()>,
}

#[derive(FromVariant)]
#[darling(attributes(drive))]
struct MyVariant {
    ident: Ident,
    fields: Fields<MyField>,
    skip: Option<()>,
}

#[derive(FromField)]
#[darling(attributes(drive))]
struct MyField {
    ident: Option<Ident>,
    ty: Type,
    skip: Option<()>,
}

pub fn impl_drive(input: DeriveInput, mutable: bool) -> Result<TokenStream> {
    let names = Names::new(mutable);
    let Names {
        visitor_trait,
        visit_trait,
        drive_trait,
        drive_inner_method,
        visitor_param,
        lifetime_param,
        mut_modifier,
        control_flow,
        ..
    } = &names;

    let input = MyTypeDecl::from_derive_input(&input)?;

    let name = &input.ident;
    let (_, ty_generics, _) = input.generics.split_for_impl();
    let impl_subject = quote! { #name #ty_generics };

    let mut generics = input.generics.clone();
    generics
        .params
        .push(GenericParam::Lifetime(parse_quote!(#lifetime_param)));
    generics
        .params
        .push(GenericParam::Type(parse_quote!(#visitor_param)));

    let where_clause = generics.make_where_clause();
    // Add `V: Visitor` so we can name `V::Break` even for a unit struct.
    where_clause
        .predicates
        .push(parse_quote!(#visitor_param: #visitor_trait));
    // Adds a `V: Visit<'s, FieldTy>` clause for each field.
    let mut need_visit_type = |f: &MyField| {
        let field_ty = &f.ty;
        where_clause
            .predicates
            .push(parse_quote!(#visitor_param: #visit_trait<#lifetime_param, #field_ty>));
    };

    let arms = match input.data {
        _ if input.skip.is_some() => quote!(),
        Data::Struct(fields) => {
            match_variant(&names, parse_quote!(Self), fields.iter(), need_visit_type)
        }
        Data::Enum(variants) => variants
            .iter()
            .filter(|variant| variant.skip.is_none())
            .map(|variant| {
                let name = &variant.ident;
                match_variant(
                    &names,
                    parse_quote!(Self::#name),
                    variant.fields.iter(),
                    &mut need_visit_type,
                )
            })
            .collect(),
    };

    let (impl_generics, _, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #drive_trait<#lifetime_param, #visitor_param> for #impl_subject
        #where_clause {
            #[allow(non_shorthand_field_patterns, unused_variables)]
            fn #drive_inner_method(&#lifetime_param #mut_modifier self, visitor: &mut #visitor_param)
                    -> #control_flow<#visitor_param::Break> {
                match self {
                    #arms
                    _ => {}
                }
                #control_flow::Continue(())
            }
        }
    })
}

/// Generate a match arm that destructures the fields of the given variant and visits each of these
/// fields.
fn match_variant<'a>(
    names: &Names,
    name: Path,
    fields: impl Iterator<Item = &'a MyField>,
    mut for_each_field: impl FnMut(&'a MyField),
) -> TokenStream {
    let visitor_param = &names.visitor_param;
    let visit_trait = &names.visit_trait;
    let (destructuring, visit_fields): (TokenStream, TokenStream) = fields
        .enumerate()
        .filter(|(_, field)| field.skip.is_none())
        .map(|(index, field)| {
            // Add a where clause to ensure this type can be visited.
            for_each_field(field);
            let field_ty = &field.ty;
            let field_id: TokenStream = match &field.ident {
                None => Index::from(index).into_token_stream(),
                Some(name) => name.into_token_stream(),
            };
            let var: TokenStream = match &field.ident {
                None => Ident::new(&format!("i{}", index), Span::call_site()).into_token_stream(),
                Some(name) => name.into_token_stream(),
            };
            (
                // Destructure this field
                quote!( #field_id : #var, ),
                // Call `visitor.visit()` on the field.
                quote!( <#visitor_param as #visit_trait<#field_ty>>::visit(visitor, #var); ),
            )
        })
        .collect();
    quote! {
        #name { #destructuring .. } => {
            #visit_fields
        }
    }
}
