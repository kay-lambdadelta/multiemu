use super::UiOutput;
use crate::environment::{ENVIRONMENT_LOCATION, Environment, graphics::GraphicsApi};
use egui::{ComboBox, Ui};
use std::{
    fs::File,
    ops::Deref,
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

        ui.horizontal(|ui| {
            if ui.button("Save Environment").clicked() {
                let file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();

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
