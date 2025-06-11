use darling::{ast, util, FromDeriveInput, FromField};
use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{spanned::Spanned, DataStruct, DeriveInput, Fields, Type};

#[derive(FromDeriveInput, Clone)]
#[darling(attributes(select), supports(struct_named))]
pub(crate) struct SelectArgs {
    data: ast::Data<util::Ignored, SelectField>,
}

#[derive(Debug, Clone, FromField)]
#[darling(attributes(select))]
pub(crate) struct SelectField {
    ident: Option<Ident>,
    ty: Type,
    #[darling(default)]
    cursor: bool,
}

#[derive(Debug, Clone)]
struct CursorAttr {
    ident: Ident,
    cursor_iden: Ident,
    ty: Type,
}

pub fn select_impl(input: &DeriveInput) -> syn::Result<TokenStream> {
    let model_type = ModelType::try_from(input)?;

    let mut out = quote! {};

    out.extend(model_type.implement_select_trait()?);

    Ok(out.into())
}

#[derive(Debug, Clone)]
pub struct ModelType<'a> {
    input: &'a DeriveInput,
    name: Ident,
    iden_name: Ident,
    iden_fields: Vec<Ident>,
    cursor: Option<CursorAttr>,
}

impl ModelType<'_> {
    fn implement_select_trait(&self) -> syn::Result<proc_macro2::TokenStream> {
        let name = &self.name;
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();
        let iden_name = &self.iden_name;
        let iden_fields = &self.iden_fields;

        let mut out = quote! {
            #[automatically_derived]
            impl #impl_generics bodega::Select for #name #ty_generics #where_clause {
                fn select_cols() -> Vec<sea_query::DynIden> {
                    use sea_query::IntoIden;

                    vec![
                        #(#iden_name::#iden_fields.into_iden()),*
                    ]
                }
            }
        };

        if let Some(ref cursor) = self.cursor {
            let ident = &cursor.ident;
            let cursor_iden = &cursor.cursor_iden;
            let ty = &cursor.ty;

            out.extend(quote! {
                #[automatically_derived]
                impl #impl_generics bodega::Cursored for #name #ty_generics #where_clause {
                    type CursorType = #ty;

                    fn cursor_value(&self) -> Self::CursorType {
                        self.#ident.clone()
                    }

                    fn cursor_column() -> sea_query::DynIden {
                        use sea_query::IntoIden;

                        #iden_name::#cursor_iden.into_iden()
                    }
                }
            });
        }

        Ok(out)
    }
}

impl<'a> TryFrom<&'a DeriveInput> for ModelType<'a> {
    type Error = syn::Error;

    fn try_from(value: &'a DeriveInput) -> Result<Self, Self::Error> {
        match value.data {
            syn::Data::Struct(ref data) => {
                let iden_fields = extract_field_iden_idents(data)?;

                let iden_name = Ident::new(&format!("{}Iden", value.ident), Span::call_site());

                let args = match SelectArgs::from_derive_input(value) {
                    Ok(v) => v,
                    Err(e) => return Err(e.into()),
                };

                let mut cursor = None;

                args.data.map_struct_fields(|field| {
                    if field.cursor {
                        let ident = field
                            .ident
                            .expect("Should have not been possible to have an unnamed field");

                        let cursor_iden =
                            Ident::new(&ident.to_string().to_upper_camel_case(), ident.span());

                        cursor = Some(CursorAttr {
                            ident,
                            cursor_iden,
                            ty: field.ty,
                        })
                    }
                });

                Ok(Self {
                    input: value,
                    name: value.ident.clone(),
                    iden_name,
                    iden_fields,
                    cursor,
                })
            }
            _ => Err(syn::Error::new(
                value.span(),
                "Select: Only works with structs with named fields (non-tuple).",
            )),
        }
    }
}

fn extract_field_iden_idents(data: &DataStruct) -> syn::Result<Vec<Ident>> {
    match data.fields {
        Fields::Named(ref fields) => Ok(fields
            .named
            .iter()
            .filter_map(|f| {
                f.ident
                    .as_ref()
                    .map(|i| Ident::new(&i.to_string().to_upper_camel_case(), Span::call_site()))
            })
            .collect()),
        _ => Err(syn::Error::new(
            data.fields.span(),
            "Select: Structs with unnamed fields are not supported.",
        )),
    }
}
