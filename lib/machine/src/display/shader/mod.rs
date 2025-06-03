use naga::{
    Module, ShaderStage,
    valid::{Capabilities, ModuleInfo, ValidationFlags, Validator},
};
use rustc_hash::FxBuildHasher;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    any::{Any, type_name},
    borrow::Cow,
    fmt::Debug,
    marker::PhantomData,
    sync::Arc,
};
use versions::SemVer;

#[cfg(feature = "opengl")]
pub mod glsl;
#[cfg(feature = "vulkan")]
pub mod spirv;

pub trait ShaderFormat: Debug + Any {
    const NAME: &'static str;
    type Representation: Serialize + DeserializeOwned + Debug + Clone + Send + Sync + 'static;

    fn compile(
        module: &Module,
        module_info: &ModuleInfo,
        version: SemVer,
        entry_name: &str,
        stage: ShaderStage,
    ) -> Result<Self::Representation, Box<dyn std::error::Error>>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Shader<T: ShaderFormat> {
    pub module: Module,
    pub vertex: T::Representation,
    pub vertex_entry: String,
    pub fragment: T::Representation,
    pub fragment_entry: String,
}

impl<T: ShaderFormat> Clone for Shader<T> {
    fn clone(&self) -> Self {
        Self {
            module: self.module.clone(),
            vertex: self.vertex.clone(),
            vertex_entry: self.vertex_entry.clone(),
            fragment: self.fragment.clone(),
            fragment_entry: self.fragment_entry.clone(),
        }
    }
}

#[allow(clippy::type_complexity)]
#[derive(Debug)]
pub struct ShaderCache<T: ShaderFormat> {
    shaders: Arc<scc::HashCache<(Cow<'static, str>, SemVer), Arc<Shader<T>>, FxBuildHasher>>,
    _format: PhantomData<T>,
}

impl<T: ShaderFormat> Clone for ShaderCache<T> {
    fn clone(&self) -> Self {
        Self {
            shaders: self.shaders.clone(),
            _format: self._format,
        }
    }
}

impl<T: ShaderFormat> Default for ShaderCache<T> {
    fn default() -> Self {
        Self {
            shaders: scc::HashCache::with_capacity_and_hasher(0, 12, FxBuildHasher).into(),
            _format: PhantomData,
        }
    }
}

impl<T: ShaderFormat> ShaderCache<T> {
    pub fn get(
        &self,
        wgsl: impl Into<Cow<'static, str>>,
        version: impl TryInto<SemVer>,
    ) -> Result<Arc<Shader<T>>, Box<dyn std::error::Error>> {
        let Ok(version) = version.try_into() else {
            return Err("Invalid version".into());
        };
        let wgsl: Cow<'static, str> = wgsl.into();

        tracing::debug!(
            "Compiling shader \"{}\" for {} version {}",
            wgsl,
            type_name::<T>(),
            version
        );

        match self.shaders.get(&(wgsl.clone(), version.clone())) {
            Some(module) => Ok(module.clone()),
            None => {
                // Try to parse it ourself and create it
                let module = naga::front::wgsl::parse_str(&wgsl)?;
                let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
                let module_info = validator.validate(&module)?;

                let vertex_entry = module
                    .entry_points
                    .iter()
                    .find(|e| e.stage == ShaderStage::Vertex)
                    .unwrap()
                    .name
                    .clone();

                let fragment_entry = module
                    .entry_points
                    .iter()
                    .find(|e| e.stage == ShaderStage::Fragment)
                    .unwrap()
                    .name
                    .clone();

                let shader = Arc::new(Shader {
                    vertex: T::compile(
                        &module,
                        &module_info,
                        version.clone(),
                        &vertex_entry,
                        ShaderStage::Vertex,
                    )?,
                    vertex_entry,
                    fragment: T::compile(
                        &module,
                        &module_info,
                        version.clone(),
                        &fragment_entry,
                        ShaderStage::Fragment,
                    )?,
                    fragment_entry,
                    module,
                });

                let _ = self
                    .shaders
                    .put((wgsl.clone(), version.clone()), shader.clone());

                Ok(shader)
            }
        }
    }
}
