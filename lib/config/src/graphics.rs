use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, EnumIter, Display, PartialEq, Eq, Default)]
pub enum GraphicsApi {
    // TODO: Once the ui rendering backend for any hwacceled api is done, enable it here
    #[default]
    Software,
    Vulkan,
    OpenGl,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GraphicsSettings {
    pub integer_scaling: bool,
    pub vsync: bool,
    pub api: GraphicsApi,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            integer_scaling: false,
            vsync: true,
            api: GraphicsApi::default(),
        }
    }
}
