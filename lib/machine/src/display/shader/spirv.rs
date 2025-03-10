use super::ShaderFormat;

pub struct SpirvShader;

impl ShaderFormat for SpirvShader {
    type Representation = Vec<u32>;

    fn compile(
        module: &naga::Module,
        module_info: &naga::valid::ModuleInfo,
        version: versions::SemVer,
        entry_name: &str,
        stage: naga::ShaderStage,
    ) -> Result<Self::Representation, Box<dyn std::error::Error>> {
        let output = naga::back::spv::write_vec(
            module,
            module_info,
            &naga::back::spv::Options {
                lang_version: (
                    version.major.try_into().unwrap(),
                    version.minor.try_into().unwrap(),
                ),
                ..Default::default()
            },
            Some(&naga::back::spv::PipelineOptions {
                shader_stage: stage,
                entry_point: entry_name.to_string(),
            }),
        )?;

        Ok(output)
    }
}
