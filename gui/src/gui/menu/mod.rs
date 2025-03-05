use egui::{Align, CentralPanel, ComboBox, Context, Layout, ScrollArea, SidePanel};
use egui_extras::{Column, TableBuilder};
use file_browser::{FileBrowserState, SortingMethod};
use multiemu_config::Environment;
use multiemu_config::graphics::GraphicsApi;
use multiemu_rom::id::RomId;
use multiemu_rom::info::RomInfo;
use multiemu_rom::manager::{ROM_INFORMATION_TABLE, RomManager};
use redb::ReadOnlyTable;
use redb::ReadableTable;
use redb::ReadableTableMetadata;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use strum::{EnumIter, IntoEnumIterator};

mod file_browser;

pub enum UiOutput {
    Resume,
    OpenGame { path: PathBuf },
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
    environment: Arc<RwLock<Environment>>,
    rom_manager: Arc<RomManager>,
    table: ReadOnlyTable<RomId, RomInfo>,
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
            file_browser_state: FileBrowserState::new(environment_guard.file_browser_home.clone()),
            environment: {
                drop(environment_guard);
                environment
            },
            table,
            rom_manager,
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
                            if ui.button(format!("{}", item)).clicked() {
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
                        let mut new_dir = None;

                        ui.horizontal(|ui| {
                            // Iter over the path segments
                            for (index, path_segment) in
                                self.file_browser_state.directory().iter().enumerate()
                            {
                                if index != 0 {
                                    ui.label("/");
                                }

                                if ui.button(path_segment.to_str().unwrap()).clicked() {
                                    new_dir = Some(PathBuf::from_iter(
                                        self.file_browser_state.directory().iter().take(index + 1),
                                    ));
                                }
                            }

                            ui.separator();

                            if ui.button("🔄").clicked() {
                                self.file_browser_state.refresh_directory();
                            }

                            let mut selected_sorting = self.file_browser_state.get_sorting_method();
                            egui::ComboBox::from_label("Sorting")
                                .selected_text(format!("{:?}", selected_sorting))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut selected_sorting,
                                        SortingMethod::Name,
                                        "Name",
                                    );
                                    ui.selectable_value(
                                        &mut selected_sorting,
                                        SortingMethod::Date,
                                        "Date",
                                    );
                                });
                            self.file_browser_state.set_sorting_method(selected_sorting);
                        });

                        TableBuilder::new(ui)
                            .striped(true)
                            .cell_layout(Layout::top_down_justified(Align::LEFT))
                            .column(Column::remainder())
                            .header(20.0, |mut header| {
                                header.col(|ui| {
                                    ui.label("Name");
                                });
                            })
                            .body(|body| {
                                body.rows(
                                    20.0,
                                    self.file_browser_state.directory_contents().len(),
                                    |mut row| {
                                        let path = self
                                            .file_browser_state
                                            .directory_contents()
                                            .get(row.index())
                                            .unwrap();

                                        row.col(|ui| {
                                            let file_name =
                                                path.file_name().unwrap().to_str().unwrap();

                                            if ui.button(file_name).clicked() {
                                                if path.is_dir() {
                                                    new_dir = Some(path.to_path_buf());
                                                }

                                                if path.is_file() {
                                                    output = Some(UiOutput::OpenGame {
                                                        path: path.to_path_buf(),
                                                    });
                                                }
                                            }
                                        });
                                    },
                                );
                            });

                        if let Some(new_dir) = new_dir {
                            tracing::trace!("Changing directory to {:?}", new_dir);
                            self.file_browser_state.change_directory(new_dir);
                        }
                    }
                    MenuItem::Options => {
                        let mut environment_guard = self.environment.write().unwrap();

                        ui.horizontal(|ui| {
                            if ui.button("Save Config").clicked() {
                                environment_guard.save().unwrap();
                            }
                        });

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
                    MenuItem::Database => {
                        TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .column(Column::auto())
                            .column(Column::auto())
                            .column(Column::auto())
                            .header(24.0, |mut header| {
                                header.col(|ui| {
                                    ui.label("Name");
                                });
                                header.col(|ui| {
                                    ui.label("System");
                                });
                                header.col(|ui| {
                                    ui.label("ID");
                                });
                            })
                            .body(|body| {
                                body.rows(24.0, self.table.len().unwrap() as usize, |mut row| {
                                    let (rom_id, rom_info) = self
                                        .table
                                        .iter()
                                        .unwrap()
                                        .nth(row.index())
                                        .unwrap()
                                        .unwrap();

                                    let (rom_id, rom_info) = (rom_id.value(), rom_info.value());

                                    row.col(|ui| {
                                        ui.label(rom_info.name.to_string());
                                    });
                                    row.col(|ui| {
                                        ui.label(rom_info.system.to_string());
                                    });
                                    row.col(|ui| {
                                        ui.label(rom_id.to_string());
                                    });
                                });
                            });
                    }
                },
            );
        });

        output
    }
}
