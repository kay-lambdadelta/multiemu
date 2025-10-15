/// Menu software renderer
pub mod software_rendering;

use crate::gui::about::AboutState;
use egui::{
    CentralPanel, Color32, Context, FontDefinitions, FontFamily, Frame, RichText, TextStyle,
    TopBottomPanel,
};
use file_browser::FileBrowserState;
use multiemu_base::{
    environment::Environment,
    program::{ProgramMetadata, ProgramSpecification},
};
use options::OptionsState;
use palette::Srgb;
use std::{
    fmt::Display,
    sync::{Arc, RwLock},
};
use strum::{EnumIter, IntoEnumIterator};

mod about;
mod file_browser;
mod options;

#[allow(clippy::large_enum_variant)]
pub enum UiOutput {
    Resume,
    Reset,
    OpenProgram {
        program_specification: ProgramSpecification,
    },
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default, EnumIter)]
pub enum MenuItem {
    #[default]
    Home,
    FileBrowser,
    Controller,
    Options,
    About,
}

impl Display for MenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MenuItem::Home => "Home",
                MenuItem::FileBrowser => "File Browser",
                MenuItem::Options => "Options",
                MenuItem::Controller => "Controller",
                MenuItem::About => "About",
            }
        )
    }
}

impl MenuItem {
    pub fn icon(self) -> &'static str {
        match self {
            MenuItem::Home => egui_phosphor::regular::HOUSE,
            MenuItem::FileBrowser => egui_phosphor::regular::FOLDERS,
            MenuItem::Controller => egui_phosphor::regular::GAME_CONTROLLER,
            MenuItem::Options => egui_phosphor::regular::GEAR,
            MenuItem::About => egui_phosphor::regular::INFO,
        }
    }
}

#[derive(Debug)]
pub struct MenuState {
    open_menu_item: MenuItem,
    file_browser_state: FileBrowserState,
    options_state: OptionsState,
    about_state: AboutState,
}

impl MenuState {
    pub fn new(
        environment: Arc<RwLock<Environment>>,
        program_manager: Arc<ProgramMetadata>,
    ) -> Self {
        let environment_guard = environment.read().unwrap();

        Self {
            open_menu_item: MenuItem::default(),
            file_browser_state: FileBrowserState::new(
                environment_guard.file_browser_home_directory.clone(),
                program_manager.clone(),
            ),
            options_state: OptionsState::new(environment.clone()),
            about_state: AboutState::default(),
        }
    }

    pub fn run_menu(&mut self, ctx: &Context) -> Option<UiOutput> {
        let mut output = None;

        TopBottomPanel::top("menu_selection")
            .resizable(false)
            .min_height(50.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for item in MenuItem::iter() {
                        let mut item_icon = RichText::new(item.icon()).size(32.0);

                        if self.open_menu_item == item {
                            item_icon = item_icon.strong();
                        }

                        if ui
                            .button(item_icon)
                            .on_hover_text(item.to_string())
                            .clicked()
                        {
                            self.open_menu_item = item;
                        }
                    }
                });

                // Display a textual label for the open menu item in case the icon didn't clarify
                ui.label(RichText::new(self.open_menu_item.to_string()).strong());
            });

        CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                Frame::new()
                    .inner_margin(10.0)
                    .show(ui, |ui| match self.open_menu_item {
                        MenuItem::Home => {
                            if ui.button("Resume").clicked() {
                                output = Some(UiOutput::Resume);
                            }
                        }
                        MenuItem::FileBrowser => {
                            self.file_browser_state.run(&mut output, ui);
                        }
                        MenuItem::Options => {
                            self.options_state.run(&mut output, ui);
                        }
                        MenuItem::Controller => {}
                        MenuItem::About => {
                            self.about_state.run(&mut output, ui);
                        }
                    });
            });
        });

        output
    }
}

pub fn setup_theme(egui_context: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    egui_context.set_fonts(fonts);

    egui_context.style_mut(|style| {
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

/// I'm not operating two color libraries at once especially since eguis is far less powerful
fn to_egui_color(color: impl Into<Srgb<u8>>) -> Color32 {
    let color = color.into();

    Color32::from_rgb(color.red, color.green, color.blue)
}
