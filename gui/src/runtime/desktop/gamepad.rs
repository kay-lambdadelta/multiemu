use crossbeam::channel::Sender;
use gilrs::{EventType, Gilrs};
use multiemu_input::{Input, InputState};

pub fn gamepad_task(sender: Sender<(Input, InputState)>) {
    let mut gilrs_context = Gilrs::new().unwrap();

    while let Some(ev) = gilrs_context.next_event_blocking(None) {
        match ev.event {
            EventType::ButtonPressed(_, _) => {}
            EventType::ButtonRepeated(_, _) => {}
            EventType::ButtonReleased(_, _) => {}
            EventType::ButtonChanged(_, _, _) => {}
            EventType::AxisChanged(_, _, _) => {}
            EventType::Connected => {}
            EventType::Disconnected => {}
            EventType::Dropped => {}
            EventType::ForceFeedbackEffectCompleted => {}
            _ => todo!(),
        }
    }
}
