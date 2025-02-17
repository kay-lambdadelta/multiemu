use crate::compile_wgsl::type_translator::TypeTranslator;
use naga::{Module, ResourceBinding};
use quote::quote;

pub fn generate_globals(module: &Module, type_translator: &mut TypeTranslator) -> Vec<syn::Item> {
    let mut globals = Vec::new();

    for (_, global_declaration) in module.global_variables.iter() {
        let global_type = type_translator.get(global_declaration.ty);
        let global_name =
            syn::parse_str::<syn::Ident>(global_declaration.name.as_ref().unwrap()).unwrap();
        let mut bindings = Vec::default();

        if let Some(ResourceBinding { group, binding }) = global_declaration.binding.clone() {
            bindings.push(quote! {
                pub const GROUP: u32 = #group;
                pub const BINDING: u32 = #binding;
            });
        }

        globals.push(syn::parse_quote! {
            pub mod #global_name {
                use super::super::types::*;

                pub type Type = #global_type;

                #(#bindings)*
            }
        });
    }

    globals
}
