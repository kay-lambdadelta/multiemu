use color::TiaColor;
use multiemu_machine::{
    component::{Component, RuntimeEssentials},
    display::backend::{ComponentFramebuffer, RenderApi},
};
use nalgebra::{DMatrix, Point2};
use palette::Srgb;
use region::Region;
use sealed::sealed;
use serde::{Deserialize, Serialize};
use std::{
    cell::OnceCell,
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::{Arc, Mutex},
};

mod color;
pub(crate) mod config;
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
enum InputControl {
    #[default]
    Normal,
    LatchedOrDumped,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct State {
    collision_matrix: HashMap<ObjectId, HashSet<ObjectId>>,
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
    color: TiaColor,
}

#[derive(Default, Debug, Serialize, Deserialize)]
enum DelayEnableChangeBall {
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
    color: TiaColor,
}

#[derive(Default, Debug, Serialize, Deserialize)]
enum DelayChangeGraphicPlayer {
    #[default]
    Disabled,
    Enabled(Option<u8>),
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Player {
    position: ObjectPosition,
    graphic: u8,
    mirror: bool,
    delay_change_graphic: DelayChangeGraphicPlayer,
    motion: i8,
    color: TiaColor,
}

#[derive(Debug)]
pub(crate) struct Tia<R: Region, A: SupportedRenderApiTia> {
    state: Arc<Mutex<State>>,
    display_backend: OnceCell<A::Backend<R>>,
}

impl<R: Region, A: SupportedRenderApiTia> Component for Tia<R, A> {}

#[sealed]
pub(crate) trait TiaDisplayBackend<R: Region, A: SupportedRenderApiTia>:
    Debug + Sized + 'static
{
    fn new(essentials: &RuntimeEssentials<A>) -> (Self, ComponentFramebuffer<A>);
    fn draw(&self, state: &State, position: Point2<u16>, hue: u8, luminosity: u8);
    fn save_screen_contents(&self) -> DMatrix<Srgb<u8>>;
    fn load_screen_contents(&self, buffer: DMatrix<Srgb<u8>>);
    fn commit_display(&self);
}

#[sealed]
pub(crate) trait SupportedRenderApiTia: RenderApi {
    type Backend<R: Region>: TiaDisplayBackend<R, Self>;
}
