use std::{
    any::{Any, type_name},
    borrow::Cow,
    fmt::Debug,
    marker::PhantomData,
    sync::Arc,
};

use naga::{
    Module, ShaderStage,
    valid::{Capabilities, ModuleInfo, ValidationFlags, Validator},
};
use rustc_hash::FxBuildHasher;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::graphics::GraphicsVersion;

#[cfg(feature = "opengl")]
mod glsl;
#[cfg(feature = "vulkan")]
mod spirv;

#[cfg(feature = "opengl")]
pub use glsl::GlslShader;
#[cfg(feature = "vulkan")]
pub use spirv::SpirvShader;

/// A specific shader format
pub trait ShaderFormat: Debug + Any {
    /// A appropiate name for the shader format
    const NAME: &'static str;
    /// Best in memory representation for the shader format
    type Representation: Serialize + DeserializeOwned + Debug + Clone + Send + Sync + 'static;

    /// Turn a [naga] module and some other info into a [`Self::Representation`]
    fn compile(
        module: &Module,
        module_info: &ModuleInfo,
        version: GraphicsVersion,
        entry_name: &str,
        stage: ShaderStage,
    ) -> Result<Self::Representation, Box<dyn std::error::Error>>;
}

#[derive(Serialize, Deserialize, Debug)]
/// A compiled shader
pub struct Shader<T: ShaderFormat> {
    /// Naga module that describes the shader
    pub module: Module,
    /// Vertex shader
    pub vertex: T::Representation,
    /// Vertex shader entry
    pub vertex_entry: String,
    /// Fragment shader
    pub fragment: T::Representation,
    /// Fragment shader entry
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
/// Shader LRU cache
///
/// Graphics apis that require shaders should provide this as part of their
/// component initialization data
pub struct ShaderCache<T: ShaderFormat> {
    shaders:
        Arc<scc::HashCache<(Cow<'static, str>, GraphicsVersion), Arc<Shader<T>>, FxBuildHasher>>,
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
            shaders: Arc::new(scc::HashCache::with_capacity_and_hasher(
                0,
                12,
                FxBuildHasher,
            )),
            _format: PhantomData,
        }
    }
}

impl<T: ShaderFormat> ShaderCache<T> {
    /// Get a shader from the cache/compiling it
    pub fn get(
        &self,
        wgsl: impl Into<Cow<'static, str>>,
        version: GraphicsVersion,
    ) -> Result<Arc<Shader<T>>, Box<dyn std::error::Error>> {
        let wgsl: Cow<'static, str> = wgsl.into();

        tracing::debug!(
            "Compiling shader \"{}\" for {} version {:?}",
            wgsl,
            type_name::<T>(),
            version
        );

        if let Some(module) = self.shaders.get_sync(&(wgsl.clone(), version)) {
            Ok(module.clone())
        } else {
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
                    version,
                    &vertex_entry,
                    ShaderStage::Vertex,
                )?,
                vertex_entry,
                fragment: T::compile(
                    &module,
                    &module_info,
                    version,
                    &fragment_entry,
                    ShaderStage::Fragment,
                )?,
                fragment_entry,
                module,
            });

            let _ = self
                .shaders
                .put_sync((wgsl.clone(), version), shader.clone());

            Ok(shader)
        }
    }
}
