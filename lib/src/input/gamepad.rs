use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(
    Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter,
)]
/// Inputs that a gamepad could give
pub enum GamepadInput {
    FPadUp,
    FPadDown,
    FPadLeft,
    FPadRight,
    CPadUp,
    CPadDown,
    CPadLeft,
    CPadRight,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    LeftTrigger,
    RightTrigger,
    ZTrigger,
    LeftSecondaryTrigger,
    RightSecondaryTrigger,
    LeftStickUp,
    LeftStickDown,
    LeftStickLeft,
    LeftStickRight,
    RightStickUp,
    RightStickDown,
    RightStickLeft,
    RightStickRight,
}
