use data_encoding::HEXLOWER_PERMISSIVE;
use multiemu_config::Environment;
use naga::{
    Module, ShaderStage,
    valid::{Capabilities, ModuleInfo, ValidationFlags, Validator},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    any::{Any, type_name},
    borrow::Cow,
    fmt::Debug,
    fs::{File, create_dir_all},
    io::Seek,
    sync::{Arc, RwLock},
};
use versions::SemVer;

pub mod glsl;
pub mod spirv;

pub trait ShaderFormat: Any {
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

#[derive(Serialize, Deserialize)]
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

#[derive(Clone, Debug)]
pub struct ShaderCache {
    environment: Arc<RwLock<Environment>>,
}

impl ShaderCache {
    pub fn new(environment: Arc<RwLock<Environment>>) -> Self {
        Self { environment }
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

        let mut hasher = Sha256::new();
        hasher.update(wgsl.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        let hash_string = HEXLOWER_PERMISSIVE.encode(&hash);

        let environment_guard = self.environment.read().unwrap();
        let shader_path = environment_guard.shader_cache_directory.join(&hash_string);

        create_dir_all(&environment_guard.shader_cache_directory)?;
        let mut file = File::options()
            .create(true)
            .append(true)
            .open(&shader_path)?;

        match bincode::serde::decode_from_std_read(&mut file, bincode::config::standard()) {
            Ok(module) => {
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

                return Ok(Shader {
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
                });
            }
            Err(_) => {
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

                file.seek(std::io::SeekFrom::Start(0))?;
                bincode::serde::encode_into_std_write(
                    &module,
                    &mut file,
                    bincode::config::standard(),
                )?;

                return Ok(Shader {
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
                });
            }
        }
    }
}
