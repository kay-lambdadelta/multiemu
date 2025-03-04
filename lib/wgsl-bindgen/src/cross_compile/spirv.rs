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
    ) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream), Box<dyn std::error::Error>>
    {
        let vertex_shader_entry = module
            .entry_points
            .iter()
            .find(|e| e.stage == ShaderStage::Vertex)
            .unwrap()
            .name
            .clone();

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
                entry_point: vertex_shader_entry,
            }),
        )?;

        let fragment_shader_entry = module
            .entry_points
            .iter()
            .find(|e| e.stage == ShaderStage::Fragment)
            .unwrap()
            .name
            .clone();

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
                entry_point: fragment_shader_entry,
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
