use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, EnumIter, Display, PartialEq, Eq, Default)]
/// Graphics API the graphics runtime will use
pub enum GraphicsApi {
    #[cfg_attr(
        not(all(
            any(target_family = "unix", target_os = "windows"),
            not(target_os = "horizon")
        )),
        default
    )]
    Software,
    #[cfg_attr(
        all(
            any(target_family = "unix", target_os = "windows"),
            not(target_os = "horizon")
        ),
        default
    )]
    Vulkan,
    Opengl,
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
