use std::fs::File;

use egui::{ComboBox, RichText, Ui};
use strum::IntoEnumIterator;

use crate::environment::{ENVIRONMENT_LOCATION, Environment, graphics::GraphicsApi};

#[derive(Debug, Default)]
pub struct OptionsState {}

impl OptionsState {
    pub fn run(&mut self, ui: &mut Ui, environment: &mut Environment) {
        ui.horizontal_top(|ui| {
            let button_text = RichText::new(egui_phosphor::regular::FLOPPY_DISK).size(32.0);

            if ui
                .button(button_text)
                .on_hover_text("Save environment to disk")
                .clicked()
            {
                let file = File::create(&*ENVIRONMENT_LOCATION).unwrap();

                environment.save(file).unwrap();
            }
        });

        ui.separator();

        ComboBox::from_label("Graphics Api")
            .selected_text(environment.graphics_setting.api.to_string())
            .show_ui(ui, |ui| {
                for api in GraphicsApi::iter() {
                    ui.selectable_value(
                        &mut environment.graphics_setting.api,
                        api,
                        api.to_string(),
                    );
                }
            });

        ui.checkbox(&mut environment.graphics_setting.vsync, "VSync");
    }
}
