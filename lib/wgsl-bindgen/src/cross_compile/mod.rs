use naga::{Module, valid::ModuleInfo};
use proc_macro2::TokenStream;
use versions::SemVer;

pub mod glsl;
pub mod spirv;

pub trait ShaderCrossCompiler {
    fn compile(
        module: &Module,
        module_info: &ModuleInfo,
        version: SemVer,
    ) -> Result<(TokenStream, TokenStream), Box<dyn std::error::Error>>;
}
