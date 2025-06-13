use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn json_value_impl(input: &DeriveInput) -> syn::Result<TokenStream> {
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics From<#ident #ty_generics> for sea_query::Value #where_clause {
            fn from(value: #ident) -> Self {
                serde_json::to_value(value)
                    .expect("failed to convert #ident to json value")
                    .into()
            }
        }

        #[automatically_derived]
        impl #impl_generics From<&#ident #ty_generics> for sea_query::Value #where_clause {
            fn from(value: &#ident) -> Self {
                serde_json::to_value(value)
                    .expect("failed to convert #ident to json value")
                    .into()
            }
        }
    }
    .into())
}
