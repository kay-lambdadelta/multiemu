use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, EnumIter, Display, PartialEq, Eq, Default)]
/// Graphics API the graphics runtime will use
pub enum GraphicsApi {
    // TODO: Once the ui rendering backend for any hwacceled api is done, enable it here
    #[default]
    /// Software rendering, very slow
    Software,
    #[cfg(platform_desktop)]
    /// Vulkan rendering, very fast
    Vulkan,
    #[cfg(platform_desktop)]
    /// Legacy OpenGL rendering, 3.3 and up only
    OpenGl,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Graphics settings
pub struct GraphicsSettings {
    /// When scaling the display buffer to the render surface, should fractional scaling be disabled?
    pub integer_scaling: bool,
    /// Vsync
    pub vsync: bool,
    /// Api to use
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
