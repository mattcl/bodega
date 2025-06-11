use darling::FromMeta;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_quote, spanned::Spanned, Ident, ImplGenerics, ItemStruct, TypeGenerics, WhereClause,
};

#[derive(Debug, Default, Copy, Clone, FromMeta)]
pub(crate) struct UuidArgs {
    #[darling(default)]
    skip_default: bool,

    #[darling(default)]
    skip_construction: bool,

    #[darling(default)]
    skip_display: bool,

    #[darling(default)]
    skip_refs: bool,

    #[darling(default)]
    skip_store: bool,
}

pub fn uuid_id_impl(args: UuidArgs, mut input: ItemStruct) -> syn::Result<TokenStream> {
    if !args.skip_store {
        input.attrs.push(parse_quote!(#[derive(sqlx::Type)]));
        input.attrs.push(parse_quote!(#[sqlx(transparent)]));
    }

    match &input.fields {
        syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let mut out = input.to_token_stream();

            let ident = &input.ident;
            let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

            if !args.skip_default {
                out.extend(default(ident, &impl_generics, &ty_generics, where_clause)?);
            }

            if !args.skip_construction {
                out.extend(construction(
                    ident,
                    &impl_generics,
                    &ty_generics,
                    where_clause,
                )?);
            }

            if !args.skip_refs {
                out.extend(refs(ident, &impl_generics, &ty_generics, where_clause)?);
            }

            if !args.skip_display {
                out.extend(display(ident, &impl_generics, &ty_generics, where_clause)?);
            }

            if !args.skip_store {
                out.extend(query_impls(
                    ident,
                    &impl_generics,
                    &ty_generics,
                    where_clause,
                )?);
            }

            Ok(out.into())
        }
        _ => Err(syn::Error::new(
            input.span(),
            "UuidId: Only newtypes in form Foo(Uuid) are supported.",
        )),
    }
}

fn default(
    ident: &Ident,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> syn::Result<proc_macro2::TokenStream> {
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics Default for #ident #ty_generics #where_clause {
            fn default() -> Self {
                Self(uuid::Uuid::now_v7())
            }
        }
    })
}

fn construction(
    ident: &Ident,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> syn::Result<proc_macro2::TokenStream> {
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics #ident #ty_generics #where_clause {
            pub fn new(uuid: uuid::Uuid) -> Self {
                Self(uuid)
            }
        }

        #[automatically_derived]
        impl #impl_generics From<uuid::Uuid> for #ident #ty_generics #where_clause {
            fn from(value: uuid::Uuid) -> Self {
                Self::new(value)
            }
        }

        #[automatically_derived]
        impl #impl_generics From<&uuid::Uuid> for #ident #ty_generics #where_clause {
            fn from(value: &uuid::Uuid) -> Self {
                Self::new(*value)
            }
        }
    })
}

fn refs(
    ident: &Ident,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> syn::Result<proc_macro2::TokenStream> {
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics std::ops::Deref for #ident #ty_generics #where_clause {
            type Target = uuid::Uuid;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        #[automatically_derived]
        impl #impl_generics AsRef<uuid::Uuid> for #ident #ty_generics #where_clause {
            fn as_ref(&self) -> &uuid::Uuid {
                &self.0
            }
        }
    })
}

fn display(
    ident: &Ident,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> syn::Result<proc_macro2::TokenStream> {
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics std::fmt::Display for #ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }
    })
}

fn query_impls(
    ident: &Ident,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: Option<&WhereClause>,
) -> syn::Result<proc_macro2::TokenStream> {
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics From<#ident #ty_generics> for sea_query::Value #where_clause {
            fn from(value: #ident #ty_generics) -> Self {
                value.0.into()
            }
        }

        #[automatically_derived]
        impl #impl_generics From<&#ident #ty_generics> for sea_query::Value #where_clause {
            fn from(value: &#ident #ty_generics) -> Self {
                value.0.into()
            }
        }
    })
}
