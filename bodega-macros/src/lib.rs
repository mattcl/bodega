use darling::{ast::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod db_bmc;
mod helpers;
mod insert;
mod json_value;
mod select;
mod store_enum;
mod update;
mod uuid_id;

#[proc_macro_derive(Select, attributes(select))]
pub fn select(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    select::select_impl(&input).unwrap_or_else(|e| e.to_compile_error().into())
}

#[proc_macro_derive(Insert, attributes(insert))]
pub fn insert(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    insert::insert_impl(&input).unwrap_or_else(|e| e.to_compile_error().into())
}

#[proc_macro_derive(Update, attributes(update))]
pub fn update(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    update::update_impl(&input).unwrap_or_else(|e| e.to_compile_error().into())
}

#[proc_macro_derive(DbBmc, attributes(db_bmc))]
pub fn db_bmc(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    db_bmc::db_bmc_impl(&input).unwrap_or_else(|e| e.to_compile_error().into())
}

#[proc_macro_derive(JsonValue)]
pub fn json_value(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    json_value::json_value_impl(&input).unwrap_or_else(|e| e.to_compile_error().into())
}

#[proc_macro_attribute]
pub fn uuid_id(attr_args: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(attr_args.into()) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(darling::Error::from(e).write_errors()),
    };

    let args = match uuid_id::UuidArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let input = syn::parse_macro_input!(item as syn::ItemStruct);

    uuid_id::uuid_id_impl(args, input).unwrap_or_else(|e| e.to_compile_error().into())
}

#[proc_macro_attribute]
pub fn store_enum(attr_args: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(attr_args.into()) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(darling::Error::from(e).write_errors()),
    };

    let args = match store_enum::StoreEnumArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let input = syn::parse_macro_input!(item as syn::ItemEnum);

    store_enum::store_enum_impl(args, input).unwrap_or_else(|e| e.to_compile_error().into())
}
