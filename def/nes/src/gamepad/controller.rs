use multiemu::{
    component::{BuildError, Component, ComponentConfig},
    input::{Input, gamepad::GamepadInput, keyboard::KeyboardInput},
    machine::{
        builder::ComponentBuilder,
        virtual_gamepad::{VirtualGamepad, VirtualGamepadMetadata},
    },
    memory::{Address, AddressSpaceId, MemoryOperationError, ReadMemoryRecord},
    platform::Platform,
};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
};

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

#[derive(Debug)]
pub struct NesController {
    gamepad: Arc<VirtualGamepad>,
    current_read: AtomicU8,
}

impl Component for NesController {
    fn read_memory(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let current_read = self.current_read.load(Ordering::Acquire);

        let pressed = self
            .gamepad
            .get(READ_ORDER[current_read as usize])
            .as_digital(None);

        buffer[0] = if pressed { 1 } else { 0 };

        if (0..READ_ORDER.len() as u8).contains(&current_read) {
            self.current_read.store(current_read + 1, Ordering::Release);
        }

        Ok(())
    }
}

impl<P: Platform> ComponentConfig<P> for NesControllerConfig {
    type Component = NesController;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        let gamepad = create_gamepad();

        let (component_builder, _) =
            component_builder.insert_gamepad("standard-nes-controller", gamepad.clone());

        component_builder.build(NesController {
            gamepad,
            current_read: AtomicU8::new(0),
        });

        Ok(())
    }
}

#[derive(Debug)]
pub struct NesControllerConfig {
    pub controller_index: u8,
}

fn create_gamepad() -> Arc<VirtualGamepad> {
    let present_inputs = HashSet::from_iter([
        Input::Gamepad(GamepadInput::DPadUp),
        Input::Gamepad(GamepadInput::DPadDown),
        Input::Gamepad(GamepadInput::DPadLeft),
        Input::Gamepad(GamepadInput::DPadRight),
        Input::Gamepad(GamepadInput::FPadDown),
        Input::Gamepad(GamepadInput::FPadRight),
        Input::Gamepad(GamepadInput::Start),
        Input::Gamepad(GamepadInput::Select),
    ]);

    VirtualGamepad::new(VirtualGamepadMetadata {
        default_bindings: HashMap::from_iter(
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
                ]),
        ),
        present_inputs,
    })
}
