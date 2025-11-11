use bitvec::{prelude::Lsb0, view::BitView};
use multiemu_runtime::{
    component::{Component, ComponentConfig},
    input::{GamepadInput, Input, VirtualGamepad, VirtualGamepadMetadata, keyboard::KeyboardInput},
    machine::builder::ComponentBuilder,
    memory::{Address, AddressSpaceId, ReadMemoryError, WriteMemoryError},
    platform::Platform,
};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, Mutex},
};

const CONTROLLER_0: Address = 0x4016;

const READ_ORDER: [Input; 8] = [
    Input::Gamepad(GamepadInput::FPadRight),
    Input::Gamepad(GamepadInput::FPadDown),
    Input::Gamepad(GamepadInput::Select),
    Input::Gamepad(GamepadInput::Start),
    Input::Gamepad(GamepadInput::DPadUp),
    Input::Gamepad(GamepadInput::DPadDown),
    Input::Gamepad(GamepadInput::DPadLeft),
    Input::Gamepad(GamepadInput::DPadRight),
];

#[derive(Debug, Default)]
struct ControllerState {
    current_read: u8,
    strobe: bool,
}

#[derive(Debug)]
pub struct NesController {
    gamepad: Arc<VirtualGamepad>,
    state: Mutex<ControllerState>,
}

impl Component for NesController {
    fn read_memory(
        &self,
        _address: Address,
        _address_space: AddressSpaceId,
        avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
        let mut state_guard = self.state.lock().unwrap();
        let buffer_bits = buffer.view_bits_mut::<Lsb0>();

        buffer_bits.set(
            0,
            if (0..READ_ORDER.len() as u8).contains(&state_guard.current_read) {
                self.gamepad
                    .get(READ_ORDER[state_guard.current_read as usize])
                    .as_digital(None)
            } else {
                true
            },
        );

        if !avoid_side_effects
            && !state_guard.strobe
            && state_guard.current_read < READ_ORDER.len() as u8
        {
            state_guard.current_read += 1;
        }

        Ok(())
    }

    fn write_memory(
        &mut self,
        _address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryError> {
        let mut state = self.state.lock().unwrap();
        state.strobe = buffer.view_bits::<Lsb0>()[0];

        if state.strobe {
            state.current_read = 0;
        }

        Ok(())
    }
}

impl<P: Platform> ComponentConfig<P> for NesControllerConfig {
    type Component = NesController;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        let gamepad = create_gamepad();

        let (component_builder, _) = component_builder
            .insert_gamepad(format!("player-{}", self.controller_index), gamepad.clone());

        let register_location = CONTROLLER_0 + self.controller_index as usize;
        let component_builder = component_builder.memory_map_read(
            self.cpu_address_space,
            register_location..=register_location,
        );

        // FIXME: The two controllers need to share this state

        component_builder.memory_map_write(self.cpu_address_space, CONTROLLER_0..=CONTROLLER_0);

        Ok(NesController {
            gamepad,
            state: Mutex::default(),
        })
    }
}

#[derive(Debug)]
pub struct NesControllerConfig {
    pub cpu_address_space: AddressSpaceId,
    pub controller_index: u8,
}

fn create_gamepad() -> Arc<VirtualGamepad> {
    let present_inputs = Vec::from_iter([
        Input::Gamepad(GamepadInput::DPadUp),
        Input::Gamepad(GamepadInput::DPadDown),
        Input::Gamepad(GamepadInput::DPadLeft),
        Input::Gamepad(GamepadInput::DPadRight),
        Input::Gamepad(GamepadInput::FPadDown),
        Input::Gamepad(GamepadInput::FPadRight),
        Input::Gamepad(GamepadInput::Start),
        Input::Gamepad(GamepadInput::Select),
    ]);

    VirtualGamepad::new(Cow::Owned(VirtualGamepadMetadata {
        default_real2virtual_mappings: HashMap::from_iter(
            present_inputs
                .iter()
                .copied()
                // NES is a pretty standard gamepad
                .map(|input| (input, input))
                .chain([
                    (
                        Input::Keyboard(KeyboardInput::ArrowDown),
                        Input::Gamepad(GamepadInput::DPadDown),
                    ),
                    (
                        Input::Keyboard(KeyboardInput::ArrowUp),
                        Input::Gamepad(GamepadInput::DPadUp),
                    ),
                    (
                        Input::Keyboard(KeyboardInput::ArrowLeft),
                        Input::Gamepad(GamepadInput::DPadLeft),
                    ),
                    (
                        Input::Keyboard(KeyboardInput::ArrowRight),
                        Input::Gamepad(GamepadInput::DPadRight),
                    ),
                    (
                        Input::Keyboard(KeyboardInput::KeyZ),
                        Input::Gamepad(GamepadInput::FPadDown),
                    ),
                    (
                        Input::Keyboard(KeyboardInput::KeyX),
                        Input::Gamepad(GamepadInput::FPadRight),
                    ),
                    (
                        Input::Keyboard(KeyboardInput::Enter),
                        Input::Gamepad(GamepadInput::Start),
                    ),
                    (
                        Input::Keyboard(KeyboardInput::ShiftRight),
                        Input::Gamepad(GamepadInput::Select),
                    ),
                ]),
        ),
        present_inputs,
    }))
}
