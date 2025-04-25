use downcast_rs::Downcast;
use memory::MemoryCallback;
use multiemu_definition_m6502::M6502;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
};
use nalgebra::{DMatrix, Point2};
use palette::Srgba;
use petgraph::prelude::UnGraphMap;
use region::Region;
use std::{
    cell::OnceCell,
    collections::HashMap,
    sync::{Arc, Mutex},
};
use strum::{EnumIter, FromRepr, IntoEnumIterator};
use task::TiaTask;

mod memory;
pub mod region;
mod software;
mod task;
#[cfg(all(feature = "vulkan", platform_desktop))]
mod vulkan;

use crate::CPU_ADDRESS_SPACE;

const HBLANK_LENGTH: u8 = 68;
const VISIBLE_SCANLINE_LENGTH: u8 = 160;
const SCANLINE_LENGTH: u8 = HBLANK_LENGTH + VISIBLE_SCANLINE_LENGTH;

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromRepr)]
enum ReadRegisters {
    Cxm0p = 0x000,
    Cxm1p = 0x001,
    Cxp0fb = 0x002,
    Cxp1fb = 0x003,
    Cxm0fb = 0x004,
    Cxm1fb = 0x005,
    Cxblpf = 0x006,
    Cxppmm = 0x007,
    Inpt0 = 0x008,
    Inpt1 = 0x009,
    Inpt2 = 0x00a,
    Inpt3 = 0x00b,
    Inpt4 = 0x00c,
    Inpt5 = 0x00d,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromRepr)]
enum WriteRegisters {
    Vsync = 0x000,
    Vblank = 0x001,
    Wsync = 0x002,
    Rsync = 0x003,
    Nusiz0 = 0x004,
    Nusiz1 = 0x005,
    Colup0 = 0x006,
    Colup1 = 0x007,
    Colupf = 0x008,
    Colubk = 0x009,
    Ctrlpf = 0x00a,
    Refp0 = 0x00b,
    Refp1 = 0x00c,
    Pf0 = 0x00d,
    Pf1 = 0x00e,
    Pf2 = 0x00f,
    Resp0 = 0x010,
    Resp1 = 0x011,
    Resm0 = 0x012,
    Resm1 = 0x013,
    Resbl = 0x014,
    Audc0 = 0x015,
    Audc1 = 0x016,
    Audf0 = 0x017,
    Audf1 = 0x018,
    Audv0 = 0x019,
    Audv1 = 0x01a,
    Grp0 = 0x01b,
    Grp1 = 0x01c,
    Enam0 = 0x01d,
    Enam1 = 0x01e,
    Enabl = 0x01f,
    Hmp0 = 0x020,
    Hmp1 = 0x021,
    Hmm0 = 0x022,
    Hmm1 = 0x023,
    Hmbl = 0x024,
    Vdelp0 = 0x025,
    Vdelp1 = 0x026,
    Vdelpl = 0x027,
    Resmp0 = 0x028,
    Resmp1 = 0x029,
    Hmove = 0x02a,
    Hmclr = 0x02b,
    Cxclr = 0x02c,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, FromRepr, EnumIter)]
enum ObjectId {
    Player0,
    Player1,
    Missile0,
    Missile1,
    Playfield,
    Ball,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Hash, Clone, Copy)]
enum ObjectPosition {
    LockedToPlayer,
    Position(Point2<u8>),
}

impl Default for ObjectPosition {
    fn default() -> Self {
        Self::Position(Point2::new(0, 0))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InputControl {
    Normal,
    LatchedOrDumped,
}

#[derive(Debug)]
struct State {
    objects: HashMap<ObjectId, Object>,
    collision_matrix: UnGraphMap<ObjectId, ()>,
    in_vsync: bool,
    in_vblank: bool,
    reset_rdy_on_scanline_end: bool,
    input_control: [InputControl; 6],
    horizontal_timer: u8,
    scanline: u16,
}

impl Default for State {
    fn default() -> Self {
        Self {
            collision_matrix: UnGraphMap::default(),
            objects: ObjectId::iter().map(|id| (id, Object::default())).collect(),
            horizontal_timer: 0,
            in_vsync: false,
            in_vblank: false,
            reset_rdy_on_scanline_end: false,
            input_control: [InputControl::Normal; 6],
            scanline: 0,
        }
    }
}

#[derive(Default, Debug)]
struct Object {
    position: ObjectPosition,
}

pub struct Tia<R: Region> {
    state: Arc<Mutex<State>>,
    display_backend: OnceCell<Box<dyn TiaDisplayBackend<R>>>,
}

#[derive(Debug, Clone)]
pub struct TiaConfig {
    pub processor_name: &'static str,
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
            .component_store()
            .get::<M6502>(config.processor_name)
            .expect("M6502 component not found");

        let component_builder = component_builder
            .insert_memory(
                [
                    (0x000..=0x00d, CPU_ADDRESS_SPACE),
                    (0x030..=0x03d, CPU_ADDRESS_SPACE),
                ],
                MemoryCallback {
                    state: state.clone(),
                    processor: processor.clone(),
                },
            )
            .insert_task(
                R::frequency(),
                TiaTask {
                    processor: processor.clone(),
                },
            )
            .insert_task(R::REFRESH_RATE, move |display: &Tia<R>, _period| {
                display.display_backend.get().unwrap().commit_display();
            });

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
    fn save_screen_contents(&self) -> DMatrix<Srgba<u8>>;
    fn load_screen_contents(&self, buffer: DMatrix<Srgba<u8>>);
    fn commit_display(&self);
}
