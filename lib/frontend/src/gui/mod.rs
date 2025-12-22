/// Menu software renderer
pub mod software_rendering;

use std::fmt::Display;

use egui::{
    CentralPanel, Color32, Context, FontDefinitions, FontFamily, Frame, FullOutput, RawInput,
    RichText, TextStyle, TopBottomPanel, Ui,
};
use file_browser::FileBrowserState;
use fluxemu_runtime::program::ProgramSpecification;
use options::OptionsState;
use palette::{Srgba, WithAlpha, named::BLACK};
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    EguiWindowingIntegration, Frontend, PlatformExt,
    environment::Environment,
    gui::{about::AboutState, gamepad_config::GamepadConfigState},
};

mod about;
mod file_browser;
mod gamepad_config;
mod options;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default, EnumIter)]
pub enum MenuItem {
    #[default]
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
                MenuItem::FileBrowser => "File Browser",
                MenuItem::Options => "Options/Environment",
                MenuItem::Controller => "Controller",
                MenuItem::About => "About",
            }
        )
    }
}

impl MenuItem {
    pub fn icon(self) -> &'static str {
        match self {
            Self::FileBrowser => egui_phosphor::regular::FOLDERS,
            Self::Controller => egui_phosphor::regular::GAME_CONTROLLER,
            Self::Options => egui_phosphor::regular::GEAR,
            Self::About => egui_phosphor::regular::INFO,
        }
    }
}

#[derive(Debug)]
pub struct GuiState<P: PlatformExt> {
    open_menu_item: MenuItem,
    file_browser_state: FileBrowserState,
    options_state: OptionsState,
    about_state: AboutState,
    gamepad_config_state: GamepadConfigState,
    context: Context,
    windowing_integration: Option<P::EguiWindowingIntegration>,
    pub active: bool,
}

impl<P: PlatformExt> GuiState<P> {
    pub fn new(environment: &Environment) -> Self {
        let context = setup_egui_context();

        Self {
            open_menu_item: MenuItem::default(),
            file_browser_state: FileBrowserState::new(
                environment.file_browser_home_directory.clone(),
            ),
            options_state: OptionsState::default(),
            about_state: AboutState::default(),
            gamepad_config_state: GamepadConfigState::new(),
            context,
            windowing_integration: None,
            active: true,
        }
    }

    /// Reinitialize the egui context
    pub fn set_windowing_integration(
        &mut self,
        mut windowing_integration: P::EguiWindowingIntegration,
    ) {
        let context = setup_egui_context();
        windowing_integration.set_egui_context(&context);
        self.windowing_integration = Some(windowing_integration);
        self.context = context;
    }

    pub fn get_windowing_integration(&mut self) -> Option<&mut P::EguiWindowingIntegration> {
        self.windowing_integration.as_mut()
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn reset_context(&mut self) {
        self.context = setup_egui_context();

        self.windowing_integration
            .as_mut()
            .unwrap()
            .set_egui_context(&self.context);
    }
}

pub struct MenuOutput {
    pub egui_output: FullOutput,
    pub new_program: Option<ProgramSpecification>,
}

impl<P: PlatformExt> Frontend<P> {
    pub fn run_menu(&mut self, input: RawInput) -> MenuOutput {
        let mut new_program = None;

        let full_output = self.gui.context.clone().run(input, |ctx| {
            if self.gui.active {
                TopBottomPanel::top("menu_selection")
                    .resizable(false)
                    .min_height(50.0)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            for item in MenuItem::iter() {
                                let mut item_icon = RichText::new(item.icon()).size(32.0);

                                if self.gui.open_menu_item == item {
                                    item_icon = item_icon.strong();
                                }

                                if ui
                                    .button(item_icon)
                                    .on_hover_text(item.to_string())
                                    .clicked()
                                {
                                    self.gui.open_menu_item = item;
                                }
                            }
                        });

                        // Display a textual label for the open menu item in case the icon didn't
                        // clarify
                        ui.label(RichText::new(self.gui.open_menu_item.to_string()).strong());
                    });

                CentralPanel::default().show(ctx, |ui| {
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        Frame::new().inner_margin(10.0).show(ui, |ui| {
                            self.menu_main(ui, &mut new_program);
                        });
                    });
                });
            } else {
                CentralPanel::default()
                    .frame(Frame {
                        fill: to_egui_color(BLACK.with_alpha(0)),
                        ..Default::default()
                    })
                    .show(ctx, |ui| {});
            }
        });

        MenuOutput {
            egui_output: full_output,
            new_program,
        }
    }

    fn menu_main(&mut self, ui: &mut Ui, new_program: &mut Option<ProgramSpecification>) {
        match self.gui.open_menu_item {
            MenuItem::FileBrowser => {
                if let Some(output) = self.gui.file_browser_state.run(ui, &self.program_manager) {
                    new_program.get_or_insert(output);
                }
            }
            MenuItem::Options => {
                self.gui.options_state.run(ui, &mut self.environment);
            }
            MenuItem::Controller => {
                self.gui.gamepad_config_state.run(ui);
            }
            MenuItem::About => {
                self.gui.about_state.run(ui);
            }
        }
    }
}

pub fn setup_egui_context() -> Context {
    let egui_context = Context::default();
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

    egui_context
}

/// I'm not operating two color libraries at once especially since eguis is far
/// less powerful
fn to_egui_color(color: impl Into<Srgba<u8>>) -> Color32 {
    let color = color.into();

    Color32::from_rgba_unmultiplied(color.red, color.green, color.blue, color.alpha)
}
