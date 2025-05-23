use super::ShaderFormat;
use naga::{Module, ShaderStage, valid::ModuleInfo};
use versions::SemVer;

pub struct GlslShader;

impl ShaderFormat for GlslShader {
    const NAME: &'static str = "glsl";
    type Representation = String;

    fn compile(
        module: &Module,
        module_info: &ModuleInfo,
        version: SemVer,
        entry_name: &str,
        stage: ShaderStage,
    ) -> Result<Self::Representation, Box<dyn std::error::Error>> {
        let mut output = String::default();

        naga::back::glsl::Writer::new(
            &mut output,
            module,
            module_info,
            &naga::back::glsl::Options {
                version: naga::back::glsl::Version::Desktop(
                    format!("{}{}{}", version.major, version.minor, version.patch)
                        .parse()
                        .unwrap(),
                ),
                writer_flags: naga::back::glsl::WriterFlags::INCLUDE_UNUSED_ITEMS,
                ..Default::default()
            },
            &naga::back::glsl::PipelineOptions {
                shader_stage: stage,
                entry_point: entry_name.to_string(),
                multiview: None,
            },
            Default::default(),
        )?
        .write()?;

        Ok(output)
    }
}
