use darling::{ast, util, FromDeriveInput, FromField};
use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, DeriveInput, Ident, Path, Type};

use crate::helpers::option_kind;

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(update), supports(struct_named))]
pub(crate) struct UpdateArgs {
    iden_enum: Path,
    data: ast::Data<util::Ignored, UpdateField>,
}

#[derive(Debug, Clone, FromField)]
#[darling(attributes(update))]
pub(crate) struct UpdateField {
    ident: Option<Ident>,
    ty: Type,
    #[darling(default)]
    iden: Option<Path>,
}

pub fn update_impl(input: &DeriveInput) -> syn::Result<TokenStream> {
    let mut out = quote! {};

    let args = match UpdateArgs::from_derive_input(input) {
        Ok(v) => v,
        Err(e) => return Err(e.into()),
    };

    let info = UpdateInfo { input, args: &args };

    out.extend(info.implement_update_trait()?);

    Ok(out.into())
}

#[derive(Debug, Clone)]
struct UpdateInfo<'a> {
    input: &'a DeriveInput,
    args: &'a UpdateArgs,
}

impl<'a> UpdateInfo<'a> {
    fn implement_update_trait(&self) -> syn::Result<proc_macro2::TokenStream> {
        let name = &self.input.ident;
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();

        let mut body = quote! {};
        self.args.data.as_ref().map_struct_fields(|field| {
            let is_option = option_kind(&field.ty).is_some();

            let iden = if let Some(iden) = field.iden.clone() {
                iden
            } else {
                let ident = Ident::new(
                    &field
                        .ident
                        .as_ref()
                        .map(|i| i.to_string().to_upper_camel_case())
                        .expect("Only named structs supported"),
                    field.ident.span(),
                );
                let mut working = self.args.iden_enum.clone();
                working.segments.push(syn::PathSegment {
                    ident: ident.clone(),
                    arguments: syn::PathArguments::None,
                });
                working
            };

            let ident = &field.ident;
            if is_option {
                body.extend(quote! {
                    if let Some(val) = self.#ident {
                        out.push((#iden.into_iden(), val.into()));
                    }
                });
            } else {
                body.extend(quote! {
                    out.push((#iden.into_iden(), self.#ident.into()));
                });
            }
        });

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics bodega::Update for #name #ty_generics #where_clause {
                fn update_values(self) -> Vec<(sea_query::DynIden, sea_query::SimpleExpr)> {
                    use sea_query::IntoIden;

                    let mut out = Vec::default();

                    #body

                    out
                }
            }
        })
    }
}
