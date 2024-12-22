use darling::{FromDeriveInput, FromField, FromVariant};
use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use std::iter::IntoIterator;
use syn::token::Mut;
use syn::{
    parse_quote, Data, DeriveInput, Error, Field, GenericParam, Ident, Index, Lifetime, Path,
    Result, WhereClause,
};

#[derive(FromDeriveInput)]
#[darling(attributes(drive))]
struct TypeAttrs {
    skip: Option<()>,
}

#[derive(FromVariant)]
#[darling(attributes(drive))]
struct VariantAttrs {
    skip: Option<()>,
}

#[derive(FromField)]
#[darling(attributes(drive))]
struct FieldAttrs {
    skip: Option<()>,
}

struct Ctx<'a> {
    visit_trait: &'a Path,
    visitor_param: &'a Ident,
    lifetime_param: &'a Lifetime,
    where_clause: &'a mut WhereClause,
}

pub fn impl_drive(input: DeriveInput, mutable: bool) -> Result<TokenStream> {
    let attrs = TypeAttrs::from_derive_input(&input)?;

    let crate_path: Path = parse_quote! { ::derive_generic_visitor };
    let visitor_trait: Path = parse_quote!( #crate_path::Visitor );
    let visit_trait: Path = if mutable {
        parse_quote!( #crate_path::VisitMut )
    } else {
        parse_quote!( #crate_path::Visit )
    };
    let drive_trait: Path = if mutable {
        parse_quote!( #crate_path::DriveMut )
    } else {
        parse_quote!( #crate_path::Drive )
    };
    let method = Ident::new(
        if mutable {
            "drive_inner_mut"
        } else {
            "drive_inner"
        },
        Span::call_site(),
    );

    let visitor_param = Ident::new("V", Span::call_site());
    let lifetime_param: Lifetime = parse_quote!('s);
    let mut_modifier = if mutable {
        Some(Mut(Span::call_site()))
    } else {
        None
    };

    let name = input.ident;
    let (_, ty_generics, _) = input.generics.split_for_impl();
    let impl_subject = quote! { #name #ty_generics };

    let mut generics = input.generics.clone();
    generics
        .params
        .push(GenericParam::Lifetime(parse_quote!(#lifetime_param)));
    generics
        .params
        .push(GenericParam::Type(parse_quote!(#visitor_param)));
    // We will add `V: Visit<'s, FieldTy>` clauses for each field.
    let where_clause = generics.make_where_clause();
    // Add `V: Visitor` so we can name `V::Break` even for a unit struct.
    where_clause
        .predicates
        .push(parse_quote!(#visitor_param: #visitor_trait));

    let mut ctx = Ctx {
        visit_trait: &visit_trait,
        visitor_param: &visitor_param,
        lifetime_param: &lifetime_param,
        where_clause,
    };
    let arms = match input.data {
        _ if attrs.skip.is_some() => quote!(),
        Data::Struct(struct_) => {
            match_variant(&mut ctx, &parse_quote!(Self), struct_.fields.iter())?
        }
        Data::Enum(enum_) => enum_
            .variants
            .into_iter()
            .map(|x| {
                let attrs = VariantAttrs::from_variant(&x)?;
                if attrs.skip.is_some() {
                    return Ok(TokenStream::new());
                }
                let name = x.ident;
                match_variant(&mut ctx, &parse_quote!(Self::#name), x.fields.iter())
            })
            .try_collect()?,
        Data::Union(union_) => {
            return Err(Error::new_spanned(
                union_.union_token,
                "unions are not supported",
            ));
        }
    };

    let (impl_generics, _, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #drive_trait<#lifetime_param, #visitor_param> for #impl_subject
        #where_clause {
            #[allow(non_shorthand_field_patterns, unused_variables)]
            fn #method(&#lifetime_param #mut_modifier self, visitor: &mut #visitor_param)
                    -> ::std::ops::ControlFlow<#visitor_param::Break> {
                match self {
                    #arms
                    _ => {}
                }
                ::std::ops::ControlFlow::Continue(())
            }
        }
    })
}

/// Generate a match arm that destructures the fields of the given variant and visits each of these
/// fields.
fn match_variant<'a>(
    ctx: &mut Ctx<'_>,
    name: &Path,
    fields: impl Iterator<Item = &'a Field>,
) -> Result<TokenStream> {
    let (destructuring, visit_fields): (TokenStream, TokenStream) = fields
        .enumerate()
        .map(|(index, field)| {
            let field_id: TokenStream = match &field.ident {
                None => Index::from(index).into_token_stream(),
                Some(name) => name.into_token_stream(),
            };
            let var: TokenStream = match &field.ident {
                None => Ident::new(&format!("i{}", index), Span::call_site()).into_token_stream(),
                Some(name) => name.into_token_stream(),
            };
            let field_pat = quote!( #field_id : #var, );
            let visit_field = visit_field(ctx, &var, field)?;
            Ok((field_pat, visit_field))
        })
        .try_collect::<_, _, Error>()?;
    Ok(quote! {
        #name { #destructuring } => {
            #visit_fields
        }
    })
}

/// Visit a single field by calling `visitor.visit()` on it. Also adds a where clause to the impl
/// to that this call is valid.
fn visit_field(ctx: &mut Ctx<'_>, value_expr: &TokenStream, field: &Field) -> Result<TokenStream> {
    let attrs = FieldAttrs::from_field(&field)?;
    if attrs.skip.is_some() {
        return Ok(TokenStream::new());
    }

    let visitor_param = ctx.visitor_param;
    let lifetime_param = ctx.lifetime_param;
    let visit_trait = ctx.visit_trait;
    let field_ty = &field.ty;
    ctx.where_clause
        .predicates
        .push(parse_quote!(#visitor_param: #visit_trait<#lifetime_param, #field_ty>));

    Ok(quote! {
        <#visitor_param as #visit_trait<#field_ty>>::visit(visitor, #value_expr);
    })
}
