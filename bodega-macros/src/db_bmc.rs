use darling::{FromDeriveInput, FromMeta};
use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_quote, spanned::Spanned, DeriveInput, Ident, Path, Visibility};

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(db_bmc), supports(any))]
pub(crate) struct BmcArgs {
    model: Path,

    id_type: Path,

    #[darling(default)]
    model_name: Option<String>,

    #[darling(default)]
    table_name: Option<String>,

    #[darling(default)]
    iden_enum: Option<Path>,

    #[darling(default)]
    id_iden: Option<Path>,

    #[darling(default)]
    error: Option<Path>,

    #[darling(default)]
    private_methods: bool,

    #[darling(default)]
    methods: MethodArgs,
}

#[derive(Debug, Default, Clone, FromMeta)]
pub(crate) struct MethodArgs {
    #[darling(default)]
    create: Option<Path>,

    #[darling(default)]
    get: bool,

    #[darling(default)]
    list: bool,

    #[darling(default)]
    list_paginated: Option<Path>,

    #[darling(default)]
    update: Option<Path>,

    #[darling(default)]
    delete: bool,

    #[darling(default)]
    count: bool,
}

pub fn db_bmc_impl(input: &DeriveInput) -> syn::Result<TokenStream> {
    let args = match BmcArgs::from_derive_input(input) {
        Ok(v) => v,
        Err(e) => return Err(e.into()),
    };

    let info = ControllerInfo::new(input, &args)?;

    let mut out = quote! {};

    out.extend(info.trait_impl()?);
    out.extend(info.crud_methods()?);

    Ok(out.into())
}

#[derive(Debug, Clone)]
struct ControllerInfo<'a> {
    input: &'a DeriveInput,
    args: &'a BmcArgs,
    model_name: String,
    table_name: String,
    iden_enum: Path,
}

impl<'a> ControllerInfo<'a> {
    fn new(input: &'a DeriveInput, args: &'a BmcArgs) -> syn::Result<Self> {
        let model_name = args
            .model_name
            .clone()
            .or_else(|| {
                args
                    .model
                    .segments.last().map(|s| s.ident.to_string().to_snake_case())
            })
            .ok_or_else(|| syn::Error::new(input.span(), "DbBmc: Failed to derive model name from model and was not provided a model_name as an argument."))?;

        // stupid way to auto-compute this
        let table_name = {
            let n = args
                .table_name
                .clone()
                .unwrap_or_else(|| model_name.clone());
            if n.ends_with('s') {
                n
            } else {
                format!("{}s", n)
            }
        };

        let iden_enum = args.iden_enum.clone().unwrap_or_else(|| {
            let mut computed = args.model.clone();

            if let Some(last) = computed.segments.last_mut() {
                let v = format!("{}Iden", &last.ident);
                last.ident = Ident::new(&v, last.span())
            }

            computed
        });

        Ok(Self {
            input,
            args,
            model_name,
            table_name,
            iden_enum,
        })
    }

    fn trait_impl(&self) -> syn::Result<proc_macro2::TokenStream> {
        let name = &self.input.ident;
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();
        let model_name = &self.model_name;
        let table_name = &self.table_name;
        let id_type = &self.args.id_type;

        let id_iden = self.args.id_iden.clone().unwrap_or_else(|| {
            let mut computed = self.iden_enum.clone();
            computed.segments.push(syn::PathSegment {
                ident: Ident::new("Id", self.iden_enum.span()),
                arguments: syn::PathArguments::None,
            });

            computed
        });

        let error = self
            .args
            .error
            .clone()
            .unwrap_or_else(|| parse_quote! { bodega::Error });

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics bodega::DbBmc for #name #ty_generics #where_clause {
                const ENTITY: &'static str = #model_name;

                const TABLE: &'static str = #table_name;

                type Error = #error;

                type IdType = #id_type;

                fn id_column() -> sea_query::DynIden {
                    use sea_query::IntoIden;
                    #id_iden.into_iden()
                }
            }
        })
    }

    fn fn_info(&self, basename: &'static str) -> (Option<Visibility>, Ident) {
        if self.args.private_methods {
            (
                None,
                Ident::new(&format!("_{}", basename), self.args.methods.create.span()),
            )
        } else {
            (
                Some(Visibility::Public(syn::token::Pub {
                    span: self.input.span(),
                })),
                Ident::new(basename, self.args.methods.create.span()),
            )
        }
    }

    fn crud_methods(&self) -> syn::Result<proc_macro2::TokenStream> {
        let name = &self.input.ident;
        let model_type = &self.args.model;
        let id_type = &self.args.id_type;
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();

        let error = self
            .args
            .error
            .clone()
            .unwrap_or_else(|| parse_quote! { bodega::Error });

        let mut out = quote! {};

        if let Some(create_type) = self.args.methods.create.as_ref() {
            let (vis, fn_name) = self.fn_info("create");

            out.extend(quote! {
                #[automatically_derived]
                impl #impl_generics #name #ty_generics #where_clause {
                    /// Create a row in the database, returning the created row.
                    #vis async fn #fn_name<X>(executor: &mut X, data: #create_type) -> std::result::Result<#model_type, #error>
                    where
                        X: bodega::AsExecutor,
                    {
                        let res = bodega::create::<Self, _, _, _>(executor, data).await?;

                        Ok(res)
                    }
                }
            });
        }

        if self.args.methods.get {
            let (vis, fn_name) = self.fn_info("get");

            out.extend(quote! {
                #[automatically_derived]
                impl #impl_generics #name #ty_generics #where_clause {
                    /// Fetch a record from the store with the given id
                    #vis async fn #fn_name<X>(executor: &mut X, id: &#id_type) -> std::result::Result<#model_type, #error>
                    where
                        X: bodega::AsExecutor,
                    {
                        let res = bodega::get::<Self, _, _>(executor, id).await?;

                        Ok(res)
                    }
                }
            });
        }

        if self.args.methods.list {
            let (vis, fn_name) = self.fn_info("list");

            out.extend(quote! {
                #[automatically_derived]
                impl #impl_generics #name #ty_generics #where_clause {
                    /// Fetch all rows from the store.
                    #vis async fn #fn_name<X>(executor: &mut X) -> std::result::Result<Vec<#model_type>, #error>
                    where
                        X: bodega::AsExecutor,
                    {
                        let res = bodega::list::<Self, _, _>(executor).await?;

                        Ok(res)
                    }
                }
            });
        }

        if let Some(filters) = self.args.methods.list_paginated.as_ref() {
            let (vis, fn_name) = self.fn_info("list_paginated");

            out.extend(quote! {
                #[automatically_derived]
                impl #impl_generics #name #ty_generics #where_clause {
                    /// Fetch all rows from the store.
                    #vis async fn #fn_name<X>(executor: &mut X, filters: &#filters) -> std::result::Result<bodega::Paginated<#model_type>, #error>
                    where
                        X: bodega::AsExecutor,
                    {
                        let res = bodega::list_paginated::<Self, _, _, _>(executor, filters).await?;

                        Ok(res)
                    }
                }
            });
        }

        if let Some(update_type) = self.args.methods.update.as_ref() {
            let (vis, fn_name) = self.fn_info("update");

            out.extend(quote! {
                #[automatically_derived]
                impl #impl_generics #name #ty_generics #where_clause {
                    /// Create a row in the database, returning the created row.
                    #vis async fn #fn_name<X>(executor: &mut X, id: &#id_type, data: #update_type) -> std::result::Result<#model_type, #error>
                    where
                        X: bodega::AsExecutor,
                    {
                        let res = bodega::update::<Self, _, _, _>(executor, id, data).await?;

                        Ok(res)
                    }
                }
            });
        }

        if self.args.methods.delete {
            let (vis, fn_name) = self.fn_info("delete");

            out.extend(quote! {
                #[automatically_derived]
                impl #impl_generics #name #ty_generics #where_clause {
                    /// Delete the given record from the store.
                    #vis async fn #fn_name<X>(executor: &mut X, id: &#id_type) -> std::result::Result<(), #error>
                    where
                        X: bodega::AsExecutor,
                    {
                        bodega::delete::<Self, _>(executor, id).await?;

                        Ok(())
                    }
                }
            });
        }

        if self.args.methods.count {
            let (vis, fn_name) = self.fn_info("count");

            out.extend(quote! {
                #[automatically_derived]
                impl #impl_generics #name #ty_generics #where_clause {
                    /// Delete the given record from the store.
                    #vis async fn #fn_name<X>(executor: &mut X) -> std::result::Result<usize, #error>
                    where
                        X: bodega::AsExecutor,
                    {
                        let res = bodega::count::<Self, _>(executor).await?;

                        Ok(res)
                    }
                }
            });
        }

        Ok(out)
    }
}
