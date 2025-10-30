use super::UiOutput;
use egui::{ComboBox, RichText, Ui};
use multiemu_runtime::environment::{ENVIRONMENT_LOCATION, Environment, graphics::GraphicsApi};
use std::{
    fs::File,
    sync::{Arc, RwLock},
};
use strum::IntoEnumIterator;

#[derive(Debug)]
pub struct OptionsState {
    environment: Arc<RwLock<Environment>>,
}

impl OptionsState {
    pub fn new(environment: Arc<RwLock<Environment>>) -> Self {
        Self { environment }
    }

    pub fn run(&mut self, _output: &mut Option<UiOutput>, ui: &mut Ui) {
        let mut environment_guard = self.environment.write().unwrap();

        ui.horizontal_top(|ui| {
            let button_text = RichText::new(egui_phosphor::regular::FLOPPY_DISK).size(32.0);

            if ui
                .button(button_text)
                .on_hover_text("Save environment to disk")
                .clicked()
            {
                let file = File::create(&*ENVIRONMENT_LOCATION).unwrap();

                environment_guard.save(file).unwrap();
            }
        });

        ui.separator();

        ComboBox::from_label("Graphics Api")
            .selected_text(environment_guard.graphics_setting.api.to_string())
            .show_ui(ui, |ui| {
                for api in GraphicsApi::iter() {
                    ui.selectable_value(
                        &mut environment_guard.graphics_setting.api,
                        api,
                        api.to_string(),
                    );
                }
            });

        ui.checkbox(&mut environment_guard.graphics_setting.vsync, "VSync");
    }
}
