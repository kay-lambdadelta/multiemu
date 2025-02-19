use naga::{Handle, Module, Scalar, Type};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;

// FIXME: This code is recursive

pub struct TypeTranslator<'a> {
    naga2rust_cache: HashMap<Handle<Type>, syn::Type>,
    module: &'a Module,
    pub structs: Vec<syn::ItemStruct>,
}

impl<'a> TypeTranslator<'a> {
    pub fn new(module: &'a Module) -> Self {
        let mut me = Self {
            naga2rust_cache: HashMap::default(),
            module,
            structs: Vec::default(),
        };

        for (handle, _) in module.types.iter() {
            let _ = me.get(handle);
        }

        me
    }

    pub fn get(&mut self, type_handle: Handle<Type>) -> Option<syn::Type> {
        if let Some(built) = self.naga2rust_cache.get(&type_handle) {
            return Some(built.clone());
        }

        let naga_type = self
            .module
            .types
            .get_handle(type_handle)
            .expect("Could not resolve naga type");

        if let Some(rust_type) = self.naga2rust(naga_type) {
            self.naga2rust_cache.insert(type_handle, rust_type.clone());

            return Some(rust_type);
        }

        None
    }

    fn naga2rust(&mut self, naga_type: &Type) -> Option<syn::Type> {
        match &naga_type.inner {
            naga::TypeInner::Array { base, size, .. }
            | naga::TypeInner::BindingArray { base, size } => {
                let base_type = self.get(*base);

                match size {
                    naga::ArraySize::Constant(size) => {
                        let size = size.get();
                        Some(syn::parse_quote!([#base_type; #size as usize]))
                    }
                    naga::ArraySize::Dynamic => Some(syn::parse_quote!(Vec<#base_type>)),
                    naga::ArraySize::Pending(_pending_array_size) => todo!(),
                }
            }
            naga::TypeInner::Struct { members, .. } => {
                let struct_name = naga_type.name.as_ref()?;
                let are_struct_members_named = members.iter().all(|member| member.name.is_some());

                let members: Option<Vec<_>> = members
                    .iter()
                    .enumerate()
                    .map(|(member_index, member)| {
                        let member_name = if are_struct_members_named {
                            let member_name = member.name.as_ref().unwrap();
                            syn::parse_str::<syn::Ident>(member_name)
                        } else {
                            syn::parse_str::<syn::Ident>(&format!("v{}", member_index))
                        };

                        let member_type = self.get(member.ty);
                        let mut attributes = TokenStream::new();

                        if let Type {
                            inner:
                                naga::TypeInner::Array {
                                    size: naga::ArraySize::Dynamic,
                                    ..
                                }
                                | naga::TypeInner::BindingArray {
                                    size: naga::ArraySize::Dynamic,
                                    ..
                                },
                            ..
                        } = self.module.types.get_handle(member.ty).unwrap()
                        {
                            attributes.extend(quote!(#[size(runtime)]))
                        }

                        member_name.ok().map(|member_name| {
                            quote! {
                                #attributes
                                pub #member_name: #member_type
                            }
                        })
                    })
                    .collect();

                let struct_name = syn::parse_str::<syn::Ident>(struct_name).ok();
                match (members, struct_name) {
                    (Some(members), Some(struct_name)) => {
                        self.structs.push(syn::parse_quote! {
                            #[allow(unused)]
                            #[derive(Debug, PartialEq, Clone, Copy, encase::ShaderType)]
                            pub struct #struct_name {
                                #(#members,)*
                            }
                        });
                        Some(syn::parse_quote!(#struct_name))
                    }
                    _ => None,
                }
            }
            naga::TypeInner::Scalar(scalar) => naga_scalar2rust(scalar),
            naga::TypeInner::Vector { size, scalar } => {
                let size = (*size) as usize;
                let scalar_type = naga_scalar2rust(scalar).unwrap();

                Some(syn::parse_quote!(nalgebra::SVector<#scalar_type, #size>))
            }
            naga::TypeInner::Matrix {
                columns,
                rows,
                scalar,
            } => {
                let columns = (*columns) as usize;
                let rows = (*rows) as usize;
                let scalar_type = naga_scalar2rust(scalar).unwrap();

                Some(syn::parse_quote!(nalgebra::SMatrix<#scalar_type, #columns, #rows>))
            }
            _ => None,
        }
    }
}

fn naga_scalar2rust(scalar: &Scalar) -> Option<syn::Type> {
    match (scalar.kind, scalar.width) {
        (naga::ScalarKind::Bool, 1) => Some(syn::parse_quote!(bool)),
        (naga::ScalarKind::Float, 4) => Some(syn::parse_quote!(f32)),
        (naga::ScalarKind::Float, 8) => Some(syn::parse_quote!(f64)),
        (naga::ScalarKind::Sint, 4) => Some(syn::parse_quote!(i32)),
        (naga::ScalarKind::Sint, 8) => Some(syn::parse_quote!(i64)),
        (naga::ScalarKind::Uint, 4) => Some(syn::parse_quote!(u32)),
        (naga::ScalarKind::Uint, 8) => Some(syn::parse_quote!(u64)),
        _ => None,
    }
}
