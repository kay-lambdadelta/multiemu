use bitvec::{array::BitArray, order::Lsb0};
use color::TiaColor;
use multiemu_machine::{
    component::{Component, RuntimeEssentials},
    display::backend::{ComponentFramebuffer, RenderApi},
};
use nalgebra::{DMatrixViewMut, Point2};
use palette::Srgba;
use region::Region;
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

const HBLANK_LENGTH: u16 = 68;
const VISIBLE_SCANLINE_LENGTH: u16 = 160;
const SCANLINE_LENGTH: u16 = HBLANK_LENGTH + VISIBLE_SCANLINE_LENGTH;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Serialize, Deserialize)]
enum ObjectId {
    Player0,
    Player1,
    Missile0,
    Missile1,
    Playfield,
    Ball,
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
    electron_beam: Point2<u16>,
    missiles: [Missile; 2],
    ball: Ball,
    players: [Player; 2],
    playfield: Playfield,
    high_playfield_ball_priority: bool,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Missile {
    position: Point2<u16>,
    enabled: bool,
    motion: i8,
    color: TiaColor,
    /// Locked to player and invisible
    locked: bool,
}

#[derive(Default, Debug, Serialize, Deserialize)]
enum DelayEnableChangeBall {
    #[default]
    Disabled,
    Enabled(Option<bool>),
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Ball {
    position: Point2<u16>,
    enabled: bool,
    delay_enable_change: DelayEnableChangeBall,
    motion: i8,
    color: TiaColor,
    size: u8,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Playfield {
    mirror: bool,
    color: TiaColor,
    score_mode: bool,
    // 20 bits
    data: BitArray<[u8; 4], Lsb0>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
enum DelayChangeGraphicPlayer {
    #[default]
    Disabled,
    Enabled(Option<u8>),
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Player {
    position: Point2<u16>,
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

pub(crate) trait FramebufferGuard {
    fn get(&mut self) -> DMatrixViewMut<'_, Srgba<u8>>;
}

pub(crate) trait TiaDisplayBackend<R: Region, A: SupportedRenderApiTia>:
    Debug + Sized + 'static
{
    fn new(essentials: &RuntimeEssentials<A>) -> (Self, ComponentFramebuffer<A>);
    fn lock_framebuffer(&self) -> impl FramebufferGuard;
    fn commit_display(&self);
}

pub(crate) trait SupportedRenderApiTia: RenderApi {
    type Backend<R: Region>: TiaDisplayBackend<R, Self>;
}
