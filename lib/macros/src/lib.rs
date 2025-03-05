use proc_macro::TokenStream;
use quote::quote;

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
