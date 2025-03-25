use egui::{FontFamily, TextStyle, TextWrapMode};
use multiemu_input::{GamepadId, Input, InputState};

mod audio;
mod gamepad;
mod keyboard;
pub mod renderer;
pub mod windowing;

fn setup_theme(egui_context: &egui::Context) {
    egui_context.style_mut(|style| {
        // Wrapping breaks tables
        style.wrap_mode = Some(TextWrapMode::Extend);

        style.text_styles.insert(
            TextStyle::Body,
            egui::FontId::new(18.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Button,
            egui::FontId::new(20.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            egui::FontId::new(24.0, FontFamily::Proportional),
        );
    });
}

pub enum RuntimeBoundMessage {
    Input {
        id: GamepadId,
        input: Input,
        state: InputState,
    },
}
