use naga::{
    Module, ShaderStage,
    valid::{Capabilities, ModuleInfo, ValidationFlags, Validator},
};
use std::{any::type_name, borrow::Cow, fmt::Debug};
use std::{
    any::{Any, TypeId},
    sync::Arc,
};
use versions::SemVer;

pub mod glsl;
pub mod spirv;

pub trait ShaderFormat: Any {
    type Representation: Debug + Clone + Send + Sync + 'static;

    fn compile(
        module: &Module,
        module_info: &ModuleInfo,
        version: SemVer,
        entry_name: &str,
        stage: ShaderStage,
    ) -> Result<Self::Representation, Box<dyn std::error::Error>>;
}

pub struct Shader<T: ShaderFormat> {
    pub module: Arc<Module>,
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

impl<T: ShaderFormat> Debug for Shader<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shader")
            .field("module", &self.module)
            .field("vertex", &self.vertex)
            .field("vertex_entry", &self.vertex_entry)
            .field("fragment", &self.fragment)
            .field("fragment_entry", &self.fragment_entry)
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct ShaderCache(
    scc::HashCache<(Cow<'static, str>, SemVer, TypeId), Box<dyn Any + Send + Sync>>,
);

impl ShaderCache {
    pub fn new(capacity: usize) -> Self {
        Self(scc::HashCache::with_capacity(0, capacity))
    }
}

impl ShaderCache {
    pub fn get<T: ShaderFormat>(
        &self,
        wgsl: impl Into<Cow<'static, str>>,
        version: SemVer,
    ) -> Result<Shader<T>, Box<dyn std::error::Error>> {
        let wgsl: Cow<'static, str> = wgsl.into();

        tracing::debug!(
            "Compiling shader \"{}\" for {} version {}",
            wgsl,
            type_name::<T>(),
            version
        );

        match self
            .0
            .entry((wgsl.clone(), version.clone(), TypeId::of::<T>()))
        {
            scc::hash_cache::Entry::Occupied(occupied_entry) => Ok(occupied_entry
                .get()
                .downcast_ref::<Shader<T>>()
                .unwrap()
                .clone()),
            scc::hash_cache::Entry::Vacant(vacant_entry) => {
                let module = Arc::new(naga::front::wgsl::parse_str(&wgsl)?);
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

                let shader = Shader {
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
                        version,
                        &fragment_entry,
                        ShaderStage::Fragment,
                    )?,
                    fragment_entry,
                    module,
                };

                vacant_entry.put_entry(Box::new(shader.clone()));

                Ok(shader)
            }
        }
    }
}
