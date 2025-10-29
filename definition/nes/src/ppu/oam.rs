use arrayvec::ArrayVec;
use bitvec::{field::BitField, order::Lsb0, view::BitView};
use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use serde_with::Bytes;
use serde_with::serde_as;

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct OamSprite {
    pub position: Point2<u8>,
    pub tile_index: u8,
    pub palette_index: u8,
    pub behind_background: bool,
    pub flip: Vector2<bool>,
}

impl OamSprite {
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        let position = Point2::new(bytes[3], bytes[0]);
        let tile_index = bytes[1];

        let attribute_bits = bytes[2].view_bits::<Lsb0>();
        let palette_index = attribute_bits[0..=1].load::<u8>();
        let priority = attribute_bits[5];
        let flip = Vector2::new(attribute_bits[6], attribute_bits[7]);

        OamSprite {
            position,
            tile_index,
            palette_index,
            behind_background: priority,
            flip,
        }
    }

    pub fn to_bytes(self) -> [u8; 4] {
        let mut bytes = [0; 4];
        bytes[0] = self.position.y;
        bytes[1] = self.tile_index;
        bytes[3] = self.position.y;

        let attribute_bits = bytes[2].view_bits_mut::<Lsb0>();

        attribute_bits[0..=1].store(self.palette_index);
        attribute_bits.set(5, self.behind_background);
        attribute_bits.set(6, self.flip.x);
        attribute_bits.set(7, self.flip.y);

        bytes
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum SpriteEvaluationState {
    InspectingY,
    Evaluating { sprite_y: u8 },
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct OamState {
    #[serde_as(as = "Bytes")]
    pub data: [u8; 256],
    pub oam_addr: u8,
    pub sprite_evaluation_state: SpriteEvaluationState,
    pub queued_sprites: ArrayVec<OamSprite, 8>,
    pub show_sprites_leftmost_pixels: bool,
    pub sprite_8x8_pattern_table_address: u16,
    pub sprite_rendering_enabled: bool,
}
