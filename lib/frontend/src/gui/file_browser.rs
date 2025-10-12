use super::UiOutput;
use egui::{Align, Layout};
use egui_extras::{Column, TableBuilder};
use multiemu_base::program::ProgramMetadata;
use std::{fs::read_dir, path::PathBuf, sync::Arc};
use strum::{Display, EnumIter};

#[derive(PartialEq, Eq, Clone, Copy, Debug, EnumIter, Display)]
pub enum SortingMethod {
    Name,
    Date,
}

#[derive(Clone, Debug)]
pub struct FileBrowserState {
    current_directory: PathBuf,
    directory_contents: Vec<PathBuf>,
    sorting_method: SortingMethod,
    reverse_sorting: bool,
    show_hidden: bool,
    program_manager: Arc<ProgramMetadata>,
}

impl FileBrowserState {
    pub fn new(home_directory: PathBuf, program_manager: Arc<ProgramMetadata>) -> Self {
        let mut me = Self {
            current_directory: PathBuf::default(),
            directory_contents: Vec::new(),
            sorting_method: SortingMethod::Name,
            reverse_sorting: false,
            show_hidden: false,
            program_manager,
        };
        me.change_directory(home_directory);
        me
    }

    pub fn run(&mut self, output: &mut Option<UiOutput>, ui: &mut egui::Ui) {
        let mut new_dir = None;

        ui.horizontal(|ui| {
            // Iter over the path segments
            for (index, path_segment) in self.current_directory.iter().enumerate() {
                if index != 0 {
                    ui.label(std::path::MAIN_SEPARATOR_STR);
                }

                if ui.button(path_segment.to_str().unwrap()).clicked() {
                    new_dir = Some(PathBuf::from_iter(
                        self.current_directory.iter().take(index + 1),
                    ));
                }
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui
                .button(egui_phosphor::regular::ARROWS_CLOCKWISE)
                .clicked()
            {
                self.refresh_directory();
            }

            let old_settings = (self.sorting_method, self.reverse_sorting, self.show_hidden);

            egui::ComboBox::from_label("Sorting")
                .selected_text(format!("{}", self.sorting_method))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.sorting_method, SortingMethod::Name, "Name");
                    ui.selectable_value(&mut self.sorting_method, SortingMethod::Date, "Date");
                });

            if ui
                .button(if self.reverse_sorting {
                    egui_phosphor::regular::ARROW_UP
                } else {
                    egui_phosphor::regular::ARROW_DOWN
                })
                .clicked()
            {
                self.reverse_sorting = !self.reverse_sorting;
            }

            ui.toggle_value(&mut self.show_hidden, "Show Hidden");

            if old_settings != (self.sorting_method, self.reverse_sorting, self.show_hidden) {
                self.refresh_directory();
            }
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
                body.rows(20.0, self.directory_contents.len(), |mut row| {
                    let path = self.directory_contents.get(row.index()).unwrap();

                    row.col(|ui| {
                        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

                        if ui.button(&file_name).clicked() {
                            if path.is_dir() {
                                new_dir = Some(path.to_path_buf());
                            }

                            if path.is_file() {
                                // Try to figure out what kind of game this is
                                if let Some(program_specification) = self
                                    .program_manager
                                    .identify_program_from_paths(std::iter::once(path.clone()))
                                    .unwrap()
                                {
                                    *output = Some(UiOutput::OpenProgram {
                                        program_specification,
                                    });
                                } else {
                                    tracing::error!("Could not identify ROM at {}", path.display());
                                }
                            }
                        }
                    });
                });
            });

        if let Some(new_dir) = new_dir {
            tracing::trace!("Changing directory to {:?}", new_dir);
            self.change_directory(new_dir);
        }
    }

    pub fn sort_contents(&mut self) {
        self.directory_contents
            .sort_by(|a, b| match self.sorting_method {
                SortingMethod::Name => a.file_name().into_iter().cmp(b.file_name()),
                SortingMethod::Date => a
                    .metadata()
                    .and_then(|m| m.modified())
                    .into_iter()
                    .cmp(b.metadata().and_then(|m| m.modified())),
            });

        if self.reverse_sorting {
            self.directory_contents.reverse();
        }
    }

    pub fn change_directory(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        assert!(path.is_dir());

        self.current_directory = path.clone();
        self.directory_contents = read_dir(path)
            .unwrap()
            .filter_map(|path| {
                if !self.show_hidden
                    && path
                        .as_ref()
                        .unwrap()
                        .file_name()
                        .to_str()?
                        .starts_with('.')
                {
                    return None;
                }

                Some(path.unwrap().path())
            })
            .collect();

        self.sort_contents();
    }

    pub fn refresh_directory(&mut self) {
        self.change_directory(self.current_directory.clone());
    }
}
