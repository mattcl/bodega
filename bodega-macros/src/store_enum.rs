use darling::FromMeta;
use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_quote, ItemEnum};

#[derive(Debug, Default, Clone, FromMeta)]
pub(crate) struct StoreEnumArgs {
    #[darling(default)]
    pg_type_name: Option<String>,
}

pub fn store_enum_impl(args: StoreEnumArgs, mut input: ItemEnum) -> syn::Result<TokenStream> {
    let pg_type = args
        .pg_type_name
        .clone()
        .unwrap_or_else(|| input.ident.to_string().to_snake_case());

    input.attrs.push(parse_quote!(#[derive(sqlx::Type)]));
    input
        .attrs
        .push(parse_quote!(#[sqlx(type_name = #pg_type)]));

    let mut out = input.to_token_stream();

    out.extend(impl_sea_query(&input, &pg_type)?);

    Ok(out.into())
}

fn impl_sea_query(input: &ItemEnum, pg_type: &str) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics From<#ident #ty_generics> for sea_query::SimpleExpr #where_clause {
            fn from(value: #ident #ty_generics) -> Self {
                sea_query::Expr::val(value.as_ref()).as_enum(sea_query::Alias::new(#pg_type))
            }
        }

        #[automatically_derived]
        impl #impl_generics From<&#ident #ty_generics> for sea_query::SimpleExpr #where_clause {
            fn from(value: &#ident #ty_generics) -> Self {
                sea_query::Expr::val(value.as_ref()).as_enum(sea_query::Alias::new(#pg_type))
            }
        }

        #[automatically_derived]
        impl #impl_generics sea_query::Nullable for #ident #ty_generics #where_clause {
            fn null() -> sea_query::Value {
                // any null will do
                sea_query::Value::String(None)
            }
        }
    })
}
