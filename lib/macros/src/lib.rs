use manifest::Manifest;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod manifest;

#[proc_macro]
pub fn platform_aliases(_args: TokenStream) -> TokenStream {
    quote! {
        cfg_aliases::cfg_aliases! {
            // Means a desktop runtime, indicates we will use winit/cpal/gilrs/vulkan. Android is considered a desktop runtime here cuz yeah
            platform_desktop: {
                all(
                    any(
                        target_family = "unix",
                        target_os = "windows"
                    ),
                    // The 3ds is marked as a unix like despite not being one
                    not(target_os = "horizon")
                )
            },
            platform_3ds: {
                target_os = "horizon"
            },
            // Mere speculative at this moment considering the rust port to the psp has not hit std support yet
            platform_psp: {
                target_os = "psp"
            },
            jit: {
                all(
                    any(
                        target_family = "unix",
                        target_os = "windows"
                    ),
                    // The 3ds is marked as a unix like despite not being one
                    not(target_os = "horizon"),
                    // Cranelift architectures supported
                    any(
                        target_arch = "x86_64",
                        target_arch = "aarch64",
                        target_arch = "riscv64",
                        target_arch = "s390x"
                    ),
                    feature = "jit"
                )
            },
        }
    }
    .into()
}

#[proc_macro]
pub fn manifest(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Manifest);

    let address_spaces = input
        .address_spaces
        .iter()
        .enumerate()
        .map(|(i, (key, _))| {
            let i: u8 = i.try_into().expect("Too many address spaces!");

            quote! {
                const #key: multiemu_machine::memory::AddressSpaceId =
                    multiemu_machine::memory::AddressSpaceId::new(#i);
            }
        })
        .collect::<proc_macro2::TokenStream>();

    let insert_address_spaces = input
        .address_spaces
        .iter()
        .map(|(key, value)| {
            quote! {
                let __machine = __machine.insert_address_space(#key, #value);
            }
        })
        .collect::<proc_macro2::TokenStream>();

    let insert_components = input
        .components
        .iter()
        .map(|(key, (component_struct_ident, component_config))| {
            quote! {
                let __machine = __machine.insert_component::<#component_struct_ident>(#key, #component_config);
            }
        })
        .collect::<proc_macro2::TokenStream>();

    let game_system = input.machine;

    quote! {
        #address_spaces

        pub fn manifest(
            user_specified_roms: std::vec::Vec<multiemu_rom::id::RomId>,
            rom_manager: std::sync::Arc<multiemu_rom::manager::RomManager>,
            environment: std::sync::Arc<std::sync::RwLock<multiemu_config::Environment>>,
        ) -> multiemu_machine::builder::MachineBuilder {
            let __machine = multiemu_machine::builder::MachineBuilder::new(
                #game_system,
                rom_manager.clone(),
                environment.clone(),
            );

            #insert_address_spaces

            #insert_components

            __machine
        }
    }
    .into()
}
