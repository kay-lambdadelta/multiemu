use downcast_rs::Downcast;
use memory::MemoryCallback;
use multiemu_definition_m6502::M6502;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    display::backend::software::SoftwareRendering,
    memory::AddressSpaceHandle,
};
use nalgebra::{DMatrix, Point2};
use palette::Srgb;
use petgraph::prelude::UnGraphMap;
use region::Region;
use serde::{Deserialize, Serialize};
use std::{
    cell::OnceCell,
    sync::{Arc, Mutex},
};
use task::TiaTask;

mod memory;
pub mod region;
mod software;
mod task;
#[cfg(all(feature = "vulkan", platform_desktop))]
mod vulkan;

const HBLANK_LENGTH: u8 = 68;
const VISIBLE_SCANLINE_LENGTH: u8 = 160;
const SCANLINE_LENGTH: u8 = HBLANK_LENGTH + VISIBLE_SCANLINE_LENGTH;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize)]
enum ObjectId {
    Player0,
    Player1,
    Missile0,
    Missile1,
    Playfield,
    Ball,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Hash, Clone, Copy, Serialize, Deserialize)]
enum ObjectPosition {
    LockedToPlayer,
    Position(Point2<u8>),
}

impl Default for ObjectPosition {
    fn default() -> Self {
        Self::Position(Point2::new(0, 0))
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum InputControl {
    #[default]
    Normal,
    LatchedOrDumped,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct State {
    collision_matrix: UnGraphMap<ObjectId, ()>,
    in_vsync: bool,
    in_vblank: bool,
    reset_rdy_on_scanline_end: bool,
    input_control: [InputControl; 6],
    horizontal_timer: u8,
    scanline: u16,
    missiles: [Missile; 2],
    ball: Ball,
    players: [Player; 2],
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Missile {
    position: ObjectPosition,
    enabled: bool,
    motion: i8,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub enum DelayEnableChangeBall {
    #[default]
    Disabled,
    Enabled(Option<bool>),
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Ball {
    position: ObjectPosition,
    enabled: bool,
    delay_enable_change: DelayEnableChangeBall,
    motion: i8,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub enum DelayChangeGraphicPlayer {
    #[default]
    Disabled,
    Enabled(Option<u8>),
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Player {
    position: ObjectPosition,
    color: u8,
    graphic: u8,
    mirror: bool,
    delay_change_graphic: DelayChangeGraphicPlayer,
    motion: i8,
}

pub struct Tia<R: Region> {
    state: Arc<Mutex<State>>,
    display_backend: OnceCell<Box<dyn TiaDisplayBackend<R>>>,
}

#[derive(Debug, Clone)]
pub struct TiaConfig {
    pub processor_name: &'static str,
    pub cpu_address_space: AddressSpaceHandle,
}

impl<R: Region> Component for Tia<R> {}

impl<R: Region> FromConfig for Tia<R> {
    type Config = TiaConfig;
    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        let state = Arc::new(Mutex::new(State::default()));
        let processor = essentials
            .component_store
            .get::<M6502>(config.processor_name)
            .expect("M6502 component not found");

        let component_builder = component_builder
            .insert_rw_memory(
                MemoryCallback {
                    state: state.clone(),
                    processor: processor.clone(),
                },
                [(config.cpu_address_space, 0x000..=0x03f)],
            )
            .insert_task(
                R::frequency(),
                TiaTask {
                    processor: processor.clone(),
                },
            )
            .insert_task(R::REFRESH_RATE, move |display: &Tia<R>, _period| {
                display.display_backend.get().unwrap().commit_display();
            })
            .set_display_config::<SoftwareRendering>(None, None, software::set_display_data);

        #[cfg(all(feature = "vulkan", platform_desktop))]
        let component_builder = {
            use multiemu_machine::display::backend::vulkan::VulkanRendering;
            component_builder.set_display_config::<VulkanRendering>(
                None,
                None,
                vulkan::set_display_data,
            )
        };

        component_builder.build(Tia {
            state,
            display_backend: OnceCell::new(),
        });
    }
}

trait TiaDisplayBackend<R: Region>: Downcast {
    fn draw(&self, position: Point2<u16>, hue: u8, luminosity: u8);
    fn save_screen_contents(&self) -> DMatrix<Srgb<u8>>;
    fn load_screen_contents(&self, buffer: DMatrix<Srgb<u8>>);
    fn commit_display(&self);
}
