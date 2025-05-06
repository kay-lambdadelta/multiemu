use bitvec::{prelude::Lsb0, view::BitView};
use multiemu_definition_misc::m6532_riot::{M6532Riot, SwchaCallback};
use multiemu_input::{Input, VirtualGamepadName, gamepad::GamepadInput};
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials, component_ref::ComponentRef},
    input::virtual_gamepad::{VirtualGamepad, VirtualGamepadMetadata},
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub struct Atari2600Joystick;

impl Component for Atari2600Joystick {}

impl FromConfig for Atari2600Joystick {
    type Config = Atari2600JoystickConfig;
    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        let m6532_riot: ComponentRef<M6532Riot> = essentials
            .component_store
            .get(&config.m6532_riot)
            .expect("Couldn't find m6532_riot");

        let player1_gamepad = create_gamepad();
        let player2_gamepad = create_gamepad();

        let component_builder =
            component_builder.insert_gamepads([player1_gamepad.clone(), player2_gamepad.clone()]);

        m6532_riot
            .interact(|riot| {
                riot.install_swcha(JoystickSwchaCallback {
                    gamepads: [player1_gamepad, player2_gamepad],
                });
            })
            .unwrap();

        component_builder.build_global(Self);
    }
}

#[derive(Debug)]
pub struct Atari2600JoystickConfig {
    pub m6532_riot: Cow<'static, str>,
}

#[derive(Debug)]
pub struct JoystickSwchaCallback {
    gamepads: [Arc<VirtualGamepad>; 2],
}

impl SwchaCallback for JoystickSwchaCallback {
    fn read_memory(&self) -> u8 {
        let mut value = 0;
        let value_bits = value.view_bits_mut::<Lsb0>();
        let (player1, player2) = value_bits.split_at_mut(4);

        player1.set(
            0,
            self.gamepads[0]
                .get(Input::Gamepad(GamepadInput::LeftStickUp))
                .as_digital(None),
        );
        player1.set(
            1,
            self.gamepads[0]
                .get(Input::Gamepad(GamepadInput::LeftStickDown))
                .as_digital(None),
        );
        player1.set(
            2,
            self.gamepads[0]
                .get(Input::Gamepad(GamepadInput::LeftStickLeft))
                .as_digital(None),
        );
        player1.set(
            3,
            self.gamepads[0]
                .get(Input::Gamepad(GamepadInput::LeftStickRight))
                .as_digital(None),
        );

        player2.set(
            0,
            self.gamepads[1]
                .get(Input::Gamepad(GamepadInput::LeftStickUp))
                .as_digital(None),
        );
        player2.set(
            1,
            self.gamepads[1]
                .get(Input::Gamepad(GamepadInput::LeftStickDown))
                .as_digital(None),
        );
        player2.set(
            2,
            self.gamepads[1]
                .get(Input::Gamepad(GamepadInput::LeftStickLeft))
                .as_digital(None),
        );
        player2.set(
            3,
            self.gamepads[1]
                .get(Input::Gamepad(GamepadInput::LeftStickRight))
                .as_digital(None),
        );

        value
    }

    fn write_memory(&self, value: u8) {}
}

fn create_gamepad() -> Arc<VirtualGamepad> {
    VirtualGamepad::new(
        VirtualGamepadName::new("Atari 2600 Joystick"),
        VirtualGamepadMetadata {
            present_inputs: HashSet::from_iter([
                Input::Gamepad(GamepadInput::LeftStickUp),
                Input::Gamepad(GamepadInput::LeftStickDown),
                Input::Gamepad(GamepadInput::LeftStickLeft),
                Input::Gamepad(GamepadInput::LeftStickRight),
                Input::Gamepad(GamepadInput::FPadDown),
            ]),
            default_bindings: HashMap::from_iter([
                (
                    Input::Gamepad(GamepadInput::LeftStickUp),
                    Input::Gamepad(GamepadInput::LeftStickUp),
                ),
                (
                    Input::Gamepad(GamepadInput::LeftStickDown),
                    Input::Gamepad(GamepadInput::LeftStickDown),
                ),
                (
                    Input::Gamepad(GamepadInput::LeftStickLeft),
                    Input::Gamepad(GamepadInput::LeftStickLeft),
                ),
                (
                    Input::Gamepad(GamepadInput::LeftStickRight),
                    Input::Gamepad(GamepadInput::LeftStickRight),
                ),
                (
                    Input::Gamepad(GamepadInput::FPadDown),
                    Input::Gamepad(GamepadInput::FPadDown),
                ),
            ]),
        },
    )
}
