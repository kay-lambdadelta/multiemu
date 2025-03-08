use super::ShaderCrossCompiler;
use naga::{
    ShaderStage,
    back::glsl::{Options, PipelineOptions, WriterFlags},
};
use proc_macro2::TokenStream;
use versions::SemVer;

pub struct GlslCrossCompiler;

impl ShaderCrossCompiler for GlslCrossCompiler {
    fn compile(
        module: &naga::Module,
        module_info: &naga::valid::ModuleInfo,
        version: SemVer,
        vertex_entry: &str,
        fragment_entry: &str,
    ) -> Result<(TokenStream, TokenStream), Box<dyn std::error::Error>> {
        let mut vertex_output_string = String::new();

        naga::back::glsl::Writer::new(
            &mut vertex_output_string,
            module,
            module_info,
            &Options {
                version: naga::back::glsl::Version::Desktop(
                    format!("{}{}{}", version.major, version.minor, version.patch)
                        .parse()
                        .unwrap(),
                ),
                writer_flags: WriterFlags::INCLUDE_UNUSED_ITEMS,
                ..Default::default()
            },
            &PipelineOptions {
                shader_stage: ShaderStage::Vertex,
                entry_point: vertex_entry.to_string(),
                multiview: None,
            },
            Default::default(),
        )?
        .write()?;

        let mut fragment_output_string = String::new();

        naga::back::glsl::Writer::new(
            &mut fragment_output_string,
            module,
            module_info,
            &Options {
                version: naga::back::glsl::Version::Desktop(
                    format!("{}{}{}", version.major, version.minor, version.patch)
                        .parse()
                        .unwrap(),
                ),
                writer_flags: WriterFlags::INCLUDE_UNUSED_ITEMS,
                ..Default::default()
            },
            &PipelineOptions {
                shader_stage: ShaderStage::Fragment,
                entry_point: fragment_entry.to_string(),
                multiview: None,
            },
            Default::default(),
        )?
        .write()?;

        Ok((
            syn::parse_quote! {
                pub const VERTEX_SHADER: &str = #vertex_output_string;
            },
            syn::parse_quote! {
                pub const FRAGMENT_SHADER: &str = #fragment_output_string;
            },
        ))
    }
}
