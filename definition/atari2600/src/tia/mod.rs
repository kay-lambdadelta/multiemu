use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::{Arc, Weak},
};

pub(crate) use backend::SupportedGraphicsApiTia;
use bitvec::{
    array::BitArray,
    order::{Lsb0, Msb0},
    view::BitView,
};
use color::TiaColor;
use multiemu_definition_mos6502::RdyFlag;
use multiemu_runtime::{
    component::{Component, ComponentPath, ResourcePath, SynchronizationContext},
    machine::Machine,
    memory::{Address, AddressSpaceId, MemoryError},
    scheduler::Period,
};
use nalgebra::Point2;
use region::Region;
use serde::{Deserialize, Serialize};

use crate::tia::{
    backend::TiaDisplayBackend,
    memory::{ReadRegisters, WriteRegisters},
};

mod backend;
mod color;
pub(crate) mod config;
mod memory;
pub mod region;

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
struct Missile {
    position: u16,
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
    position: u16,
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
    data: BitArray<[u8; 3], Lsb0>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
enum DelayChangeGraphicPlayer {
    #[default]
    Disabled,
    Enabled(Option<u8>),
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct Player {
    position: u16,
    graphic: u8,
    mirror: bool,
    delay_change_graphic: DelayChangeGraphicPlayer,
    motion: i8,
    color: TiaColor,
}

#[derive(Debug)]
pub(crate) struct Tia<R: Region, G: SupportedGraphicsApiTia> {
    collision_matrix: HashMap<ObjectId, HashSet<ObjectId>>,
    vblank_active: bool,
    cycles_waiting_for_vsync: Option<u16>,
    input_control: [InputControl; 6],
    electron_beam: Point2<u16>,
    missiles: [Missile; 2],
    ball: Ball,
    players: [Player; 2],
    playfield: Playfield,
    high_playfield_ball_priority: bool,
    background_color: TiaColor,
    backend: Option<G::Backend<R>>,
    cpu_rdy: Arc<RdyFlag>,
    machine: Weak<Machine>,
    timestamp: Period,
    my_path: ComponentPath,
}

impl<R: Region, G: SupportedGraphicsApiTia> Component for Tia<R, G> {
    fn memory_read(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        _avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), MemoryError> {
        let data = &mut buffer[0];

        if let Some(address) = ReadRegisters::from_repr(address as u16) {
            tracing::debug!("Reading from TIA register: {:?}", address);

            self.handle_read_register(data, address);

            Ok(())
        } else {
            unreachable!("{:x}", address);
        }
    }

    fn memory_write(
        &mut self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), MemoryError> {
        let data = buffer[0];
        let data_bits = data.view_bits::<Lsb0>();

        if let Some(address) = WriteRegisters::from_repr(address as u16) {
            tracing::debug!("Writing to TIA register: {:?} = {:02x}", address, data);

            self.handle_write_register(data, data_bits, address);

            Ok(())
        } else {
            unreachable!("{:x}", address);
        }
    }

    fn access_framebuffer(&mut self, _path: &ResourcePath) -> &dyn Any {
        self.backend.as_mut().unwrap().access_framebuffer()
    }

    fn synchronize(&mut self, mut context: SynchronizationContext) {
        while context.allocate_period(R::frequency().recip()) {
            self.timestamp = context.now();

            if let Some(cycles) = self.cycles_waiting_for_vsync {
                self.cycles_waiting_for_vsync = Some(cycles.saturating_sub(1));

                if self.cycles_waiting_for_vsync == Some(0) {
                    self.backend.as_mut().unwrap().commit_staging_buffer();

                    self.cycles_waiting_for_vsync = None;
                }
            }

            if !(self.cycles_waiting_for_vsync.is_some() || self.vblank_active)
                && (HBLANK_LENGTH..(VISIBLE_SCANLINE_LENGTH + HBLANK_LENGTH))
                    .contains(&self.electron_beam.x)
            {
                let color = R::color_to_srgb(self.get_rendered_color());

                self.backend
                    .as_mut()
                    .unwrap()
                    .modify_staging_buffer(|mut staging_buffer_guard| {
                        staging_buffer_guard[(
                            (self.electron_beam.x - HBLANK_LENGTH) as usize,
                            self.electron_beam.y as usize,
                        )] = color.into();
                    });
            }

            self.electron_beam.x += 1;

            if self.electron_beam.x >= SCANLINE_LENGTH {
                self.electron_beam.x = 0;
                self.electron_beam.y += 1;
            }

            if self.electron_beam.y >= R::TOTAL_SCANLINES {
                self.electron_beam.y = 0;
            }
        }
    }

    fn needs_work(&self, delta: Period) -> bool {
        delta >= R::frequency().recip()
    }
}

impl<R: Region, G: SupportedGraphicsApiTia> Tia<R, G> {
    fn get_rendered_color(&self) -> TiaColor {
        if self.high_playfield_ball_priority {
            // Check if in the bounds of ball
            if self.get_ball_color() {
                return self.ball.color;
            }

            // Check if in the bounds of playfield
            if let Some(color) = self.get_playfield_color() {
                return color;
            }

            // Check if in the bounds of player 0
            if let Some(color) = self.get_player_color(0) {
                return color;
            }

            // Check if in the bounds of player 1
            if let Some(color) = self.get_player_color(1) {
                return color;
            }

            // Check if in the bounds of missile 0
            if self.get_missile_color(0) {
                return self.missiles[0].color;
            }

            // Check if in the bounds of missile 1
            if self.get_missile_color(1) {
                return self.missiles[1].color;
            }
        } else {
            // Check if in the bounds of player 0
            if let Some(color) = self.get_player_color(0) {
                return color;
            }

            // Check if in the bounds of player 1
            if let Some(color) = self.get_player_color(1) {
                return color;
            }

            // Check if in the bounds of missile 0
            if self.get_missile_color(0) {
                return self.missiles[0].color;
            }

            // Check if in the bounds of missile 1
            if self.get_missile_color(1) {
                return self.missiles[1].color;
            }

            // Check if in the bounds of ball
            if self.get_ball_color() {
                return self.ball.color;
            }

            // Check if in the bounds of playfield
            if let Some(color) = self.get_playfield_color() {
                return color;
            }
        }

        self.background_color
    }

    #[inline]
    fn get_player_color(&self, index: usize) -> Option<TiaColor> {
        let player = &self.players[index];

        if let Some(sprite_pixel) = self
            .electron_beam
            .x
            .checked_sub(player.position)
            .map(usize::from)
        {
            if player.mirror {
                let slice = player.graphic.view_bits::<Lsb0>();

                if let Some(sprite_pixel) = slice.get(sprite_pixel).as_deref() {
                    return if *sprite_pixel {
                        Some(player.color)
                    } else {
                        None
                    };
                }
            } else {
                let slice = player.graphic.view_bits::<Msb0>();

                if let Some(sprite_pixel) = slice.get(sprite_pixel).as_deref() {
                    return if *sprite_pixel {
                        Some(player.color)
                    } else {
                        None
                    };
                }
            }
        }

        None
    }

    #[inline]
    fn get_missile_color(&self, index: usize) -> bool {
        let missile = &self.missiles[index];

        if missile.locked {
            return false;
        }

        self.electron_beam.x == missile.position
    }

    #[inline]
    fn get_ball_color(&self) -> bool {
        let ball = &self.ball;

        self.electron_beam.x == ball.position
    }

    #[inline]
    fn get_playfield_color(&self) -> Option<TiaColor> {
        let playfield_position = ((self.electron_beam.x - HBLANK_LENGTH) / 4) as usize;

        match playfield_position {
            0..20 => {
                if self.playfield.data[playfield_position] {
                    if self.playfield.score_mode {
                        Some(self.players[0].color)
                    } else {
                        Some(self.playfield.color)
                    }
                } else {
                    None
                }
            }
            20..40 => {
                let mut data = self.playfield.data;

                if self.playfield.mirror {
                    data.reverse();
                }

                if data[playfield_position - 20] {
                    if self.playfield.score_mode {
                        Some(self.players[1].color)
                    } else {
                        Some(self.playfield.color)
                    }
                } else {
                    None
                }
            }
            _ => unreachable!(),
        }
    }
}
