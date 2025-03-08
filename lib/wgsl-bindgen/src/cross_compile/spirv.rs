use super::ShaderCrossCompiler;
use naga::{
    ShaderStage,
    back::spv::{Options, PipelineOptions},
};

pub struct SpirvShaderCrossCompiler;

impl ShaderCrossCompiler for SpirvShaderCrossCompiler {
    fn compile(
        module: &naga::Module,
        module_info: &naga::valid::ModuleInfo,
        version: versions::SemVer,
        vertex_entry: &str,
        fragment_entry: &str,
    ) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), Box<dyn std::error::Error>>
    {
        let vertex_output = naga::back::spv::write_vec(
            module,
            module_info,
            &Options {
                lang_version: (
                    version.major.try_into().unwrap(),
                    version.minor.try_into().unwrap(),
                ),
                ..Default::default()
            },
            Some(&PipelineOptions {
                shader_stage: ShaderStage::Vertex,
                entry_point: vertex_entry.to_string(),
            }),
        )?;

        let fragment_output = naga::back::spv::write_vec(
            module,
            module_info,
            &Options {
                lang_version: (
                    version.major.try_into().unwrap(),
                    version.minor.try_into().unwrap(),
                ),
                ..Default::default()
            },
            Some(&PipelineOptions {
                shader_stage: ShaderStage::Fragment,
                entry_point: fragment_entry.to_string(),
            }),
        )?;

        Ok((
            syn::parse_quote!(
                pub const VERTEX_SHADER: &[u32] = &[#(#vertex_output),*];
            ),
            syn::parse_quote!(
                pub const FRAGMENT_SHADER: &[u32] = &[#(#fragment_output),*];
            ),
        ))
    }
}
