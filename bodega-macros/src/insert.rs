use darling::{ast, util, FromDeriveInput, FromField};
use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_quote, spanned::Spanned, DeriveInput, Ident, Path, Type};

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(insert), supports(struct_named))]
pub(crate) struct InsertArgs {
    iden_enum: Path,
    data: ast::Data<util::Ignored, InsertField>,
}

#[derive(Debug, Clone, FromField)]
#[darling(attributes(insert))]
pub(crate) struct InsertField {
    ident: Option<Ident>,
    #[allow(unused)]
    ty: Type,
    #[darling(default)]
    iden: Option<Path>,
    #[darling(default)]
    cust_opt: bool,
}

pub fn insert_impl(input: &DeriveInput) -> syn::Result<TokenStream> {
    let mut out = quote! {};

    let args = match InsertArgs::from_derive_input(input) {
        Ok(v) => v,
        Err(e) => return Err(e.into()),
    };

    let info = InsertInfo { input, args: &args };

    out.extend(info.implement_insert_trait()?);

    Ok(out.into())
}

#[derive(Debug, Clone)]
struct InsertInfo<'a> {
    input: &'a DeriveInput,
    args: &'a InsertArgs,
}

impl<'a> InsertInfo<'a> {
    fn implement_insert_trait(&self) -> syn::Result<proc_macro2::TokenStream> {
        let name = &self.input.ident;
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();

        let mut iden_fields = Vec::default();
        let mut inserts = Vec::default();

        self.args.data.as_ref().map_struct_fields(|field| {
            if let Some(iden) = field.iden.clone() {
                iden_fields.push(iden);
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
                iden_fields.push(working);
            }
            let ident = field.ident.as_ref().expect("Only named structs supported");

            if field.cust_opt {
                inserts.push(quote! { bodega::CustomOption(self.#ident).into() });
            } else {
                inserts.push(quote! { self.#ident.into() });
            }
        });

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics bodega::Insert for #name #ty_generics #where_clause {
                fn insert_cols(&self) -> Vec<sea_query::DynIden> {
                    use sea_query::IntoIden;

                    vec![
                        #(#iden_fields.into_iden()),*
                    ]
                }

                fn insert_vals(self) -> Vec<sea_query::SimpleExpr> {
                    vec![
                        #(#inserts),*
                    ]
                }
            }
        })
    }
}
