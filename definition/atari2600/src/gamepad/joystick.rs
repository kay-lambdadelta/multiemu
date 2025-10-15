use bitvec::{prelude::Lsb0, view::BitView};
use multiemu_base::{
    component::{Component, ComponentConfig, ComponentPath},
    input::{Input, gamepad::GamepadInput, keyboard::KeyboardInput},
    machine::{
        builder::ComponentBuilder,
        virtual_gamepad::{VirtualGamepad, VirtualGamepadMetadata},
    },
    platform::Platform,
};
use multiemu_definition_misc::mos6532_riot::{Mos6532Riot, SwchaCallback};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Debug)]
pub struct Atari2600Joystick;

impl Component for Atari2600Joystick {}

impl<P: Platform> ComponentConfig<P> for Atari2600JoystickConfig {
    type Component = Atari2600Joystick;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let player1_gamepad = create_gamepad();
        let player2_gamepad = create_gamepad();

        let (component_builder, _) =
            component_builder.insert_gamepad("atari-2600-joystick-0", player1_gamepad.clone());
        let (component_builder, _) =
            component_builder.insert_gamepad("atari-2600-joystick-1", player2_gamepad.clone());

        component_builder
            .registry()
            .interact::<Mos6532Riot, _>(&self.mos6532_riot, |riot| {
                riot.install_swcha(JoystickSwchaCallback {
                    gamepads: [player1_gamepad, player2_gamepad],
                });
            })
            .unwrap();

        Ok(Atari2600Joystick)
    }
}

#[derive(Debug)]
pub struct Atari2600JoystickConfig {
    pub mos6532_riot: ComponentPath,
}

#[derive(Debug)]
pub struct JoystickSwchaCallback {
    gamepads: [Arc<VirtualGamepad>; 2],
}

impl SwchaCallback for JoystickSwchaCallback {
    fn read_register(&self) -> u8 {
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

    fn write_register(&self, value: u8) {}
}

fn create_gamepad() -> Arc<VirtualGamepad> {
    VirtualGamepad::new(VirtualGamepadMetadata {
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
            (
                Input::Keyboard(KeyboardInput::ArrowDown),
                Input::Gamepad(GamepadInput::LeftStickDown),
            ),
            (
                Input::Keyboard(KeyboardInput::ArrowUp),
                Input::Gamepad(GamepadInput::LeftStickUp),
            ),
            (
                Input::Keyboard(KeyboardInput::ArrowLeft),
                Input::Gamepad(GamepadInput::LeftStickLeft),
            ),
            (
                Input::Keyboard(KeyboardInput::ArrowRight),
                Input::Gamepad(GamepadInput::LeftStickRight),
            ),
            (
                Input::Keyboard(KeyboardInput::KeyZ),
                Input::Gamepad(GamepadInput::FPadDown),
            ),
        ]),
    })
}
