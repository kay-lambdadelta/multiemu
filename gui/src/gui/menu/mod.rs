use database::DatabaseState;
use egui::{CentralPanel, Context, ScrollArea, SidePanel};
use file_browser::FileBrowserState;
use multiemu_config::Environment;
use multiemu_rom::id::RomId;
use multiemu_rom::info::RomInfo;
use multiemu_rom::manager::{ROM_INFORMATION_TABLE, RomManager};
use options::OptionsState;
use redb::ReadOnlyTable;
use redb::ReadableTable;
use redb::ReadableTableMetadata;
use std::fmt::Display;
use std::sync::{Arc, RwLock};
use strum::{EnumIter, IntoEnumIterator};

mod database;
mod file_browser;
mod options;

pub enum UiOutput {
    Resume,
    OpenGame { rom_id: RomId },
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Default, EnumIter)]
pub enum MenuItem {
    #[default]
    Main,
    FileBrowser,
    Options,
    Database,
}

impl Display for MenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MenuItem::Main => "Main",
                MenuItem::FileBrowser => "File Browser",
                MenuItem::Options => "Options",
                MenuItem::Database => "Database",
            }
        )
    }
}

#[derive(Debug)]
pub struct MenuState {
    open_menu_item: MenuItem,
    file_browser_state: FileBrowserState,
    options_state: OptionsState,
    database_state: DatabaseState,
    table: ReadOnlyTable<RomId, RomInfo>,
    autofocus: bool,
}

impl MenuState {
    pub fn new(environment: Arc<RwLock<Environment>>, rom_manager: Arc<RomManager>) -> Self {
        let environment_guard = environment.read().unwrap();
        let table = rom_manager
            .rom_information
            .begin_read()
            .unwrap()
            .open_table(ROM_INFORMATION_TABLE)
            .unwrap();

        Self {
            open_menu_item: MenuItem::default(),
            file_browser_state: FileBrowserState::new(
                environment_guard.file_browser_home.clone(),
                rom_manager.clone(),
            ),
            options_state: OptionsState::new(environment.clone()),
            database_state: DatabaseState::new(rom_manager),
            table,
            autofocus: true,
        }
    }

    /// TODO: barely does anything
    pub fn run_menu(&mut self, ctx: &Context) -> Option<UiOutput> {
        let mut output = None;

        SidePanel::left("options_panel")
            .resizable(true)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        for item in MenuItem::iter() {
                            if item == self.open_menu_item && self.autofocus {
                                self.autofocus = false;

                                let button = ui.button(format!("{}", item));
                                button.request_focus();

                                if button.clicked() {
                                    self.open_menu_item = item;
                                }
                            } else if ui.button(format!("{}", item)).clicked() {
                                self.autofocus = true;
                                self.open_menu_item = item;
                            }
                        }
                    })
                })
            });

        CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(
                egui::Layout::top_down_justified(egui::Align::LEFT),
                |ui| match self.open_menu_item {
                    MenuItem::Main => {
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
                    MenuItem::Database => {
                        self.database_state.run(&mut output, ui);
                    }
                },
            );
        });

        output
    }
}
