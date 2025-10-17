use super::UiOutput;
use crate::gui::to_egui_color;
use cfg_if::cfg_if;
use egui::{Button, Frame, ScrollArea, Stroke, TextEdit};
use indexmap::IndexMap;
use multiemu_runtime::program::ProgramMetadata;
use palette::{
    WithAlpha,
    named::{GREEN, RED},
};
use std::{
    fs::{self, File},
    path::PathBuf,
    sync::Arc,
    time::SystemTime,
};
use strum::{Display, EnumIter};

#[derive(PartialEq, Eq, Clone, Copy, Debug, EnumIter, Display)]
pub enum SortingMethod {
    Name,
    Date,
}

#[derive(Debug, Clone)]
pub enum PathBarState {
    Normal(PathBuf),
    Editing(String),
}

#[derive(Clone, Debug)]
pub struct FileBrowserState {
    pathbar_state: PathBarState,
    directory_contents: IndexMap<PathBuf, DirectoryContentMetadata>,
    sorting_method: SortingMethod,
    reverse_sorting: bool,
    show_hidden: bool,
    program_manager: Arc<ProgramMetadata>,
}

#[derive(Clone, Debug)]
struct DirectoryContentMetadata {
    readable: bool,
    modified: SystemTime,
    name: String,
    is_hidden: bool,
}

impl FileBrowserState {
    pub fn new(home_directory: PathBuf, program_manager: Arc<ProgramMetadata>) -> Self {
        let mut me = Self {
            pathbar_state: PathBarState::Normal(home_directory),
            directory_contents: IndexMap::default(),
            sorting_method: SortingMethod::Name,
            reverse_sorting: false,
            show_hidden: false,
            program_manager,
        };
        me.refresh_current_dir();
        me
    }

    pub fn run(&mut self, output: &mut Option<UiOutput>, ui: &mut egui::Ui) {
        let mut new_dir = None;

        ui.horizontal_top(|ui| {
            match &mut self.pathbar_state {
                PathBarState::Normal(path) => {
                    // Iter over the path segments
                    for (index, path_segment) in path.iter().enumerate() {
                        if index != 0 {
                            ui.label(std::path::MAIN_SEPARATOR_STR);
                        }

                        if ui.button(path_segment.to_string_lossy()).clicked() {
                            new_dir = Some(PathBuf::from_iter(path.iter().take(index + 1)));
                        }
                    }

                    ui.add_space(2.0);

                    if ui
                        .button(egui_phosphor::regular::PENCIL)
                        .on_hover_text("Manually edit path bar")
                        .clicked()
                    {
                        self.pathbar_state =
                            PathBarState::Editing(path.to_string_lossy().into_owned());
                    }
                }
                PathBarState::Editing(pathbar_contents) => {
                    let pathbuf = PathBuf::from(pathbar_contents.trim());

                    let is_real_dir = pathbuf.is_dir() && pathbuf.read_dir().is_ok();

                    // Check if the path the user entered is real and we can read it
                    let edit_box_frame_color =
                        if is_real_dir { GREEN } else { RED }.with_alpha(u8::MAX / 2);

                    Frame::NONE
                        .stroke(Stroke::new(4.0, to_egui_color(edit_box_frame_color)))
                        .corner_radius(2.0)
                        .inner_margin(2.0)
                        .show(ui, |ui| {
                            let mut edit = TextEdit::singleline(pathbar_contents);
                            edit = edit.desired_width(ui.available_width());

                            // Note that [TextEdit] loses focus when you press enter
                            if ui.add(edit).lost_focus() && is_real_dir {
                                new_dir = Some(pathbuf);
                            }
                        });
                }
            }
        });

        ui.separator();

        ui.horizontal_top(|ui| {
            if ui
                .button(egui_phosphor::regular::ARROWS_CLOCKWISE)
                .on_hover_text("Refresh file browser file listings")
                .clicked()
            {
                self.refresh_current_dir();
            }

            let old_settings = (self.sorting_method, self.reverse_sorting, self.show_hidden);

            if ui
                .button(if self.reverse_sorting {
                    egui_phosphor::regular::ARROW_UP
                } else {
                    egui_phosphor::regular::ARROW_DOWN
                })
                .on_hover_text("Toggle sort order")
                .clicked()
            {
                self.reverse_sorting = !self.reverse_sorting;
            }

            egui::ComboBox::from_id_salt("Sorting Method")
                .selected_text(format!("{}", self.sorting_method))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.sorting_method, SortingMethod::Name, "Name");
                    ui.selectable_value(&mut self.sorting_method, SortingMethod::Date, "Date");
                })
                .response
                .on_hover_text("Swap the file browser sorting method");

            ui.toggle_value(&mut self.show_hidden, egui_phosphor::regular::EYE_CLOSED)
                .on_hover_text("Toggle hidden file visiblity");

            if old_settings != (self.sorting_method, self.reverse_sorting, self.show_hidden) {
                self.refresh_current_dir();
            }
        });

        ScrollArea::vertical().show(ui, |ui| {
            for (path, metadata) in self.directory_contents.iter() {
                if metadata.is_hidden && !self.show_hidden {
                    continue;
                }

                let button = if metadata.readable {
                    Button::new(&metadata.name)
                } else {
                    Button::new(format!(
                        "{} {}",
                        metadata.name,
                        egui_phosphor::regular::LOCK
                    ))
                };

                if ui
                    .add_enabled(metadata.readable, button)
                    .on_disabled_hover_text(
                        "You have no read permissions for this filesystem entry",
                    )
                    .clicked()
                {
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
            }
        });

        if let Some(new_dir) = new_dir {
            tracing::trace!("Changing directory to {:?}", new_dir);
            self.pathbar_state = PathBarState::Normal(new_dir);
            self.refresh_current_dir();
        }
    }

    pub fn sort_contents(&mut self) {
        self.directory_contents
            .sort_by(|_, a, _, b| match self.sorting_method {
                SortingMethod::Name => a.name.cmp(&b.name),
                SortingMethod::Date => a.modified.cmp(&b.modified),
            });

        if self.reverse_sorting {
            self.directory_contents.reverse();
        }
    }

    pub fn refresh_current_dir(&mut self) {
        if let PathBarState::Normal(path) = &self.pathbar_state {
            let directory_contents = match fs::read_dir(path) {
                Ok(directory_contents) => directory_contents.filter_map(|dir_entry| {
                    // Don't care about items we can't iter over
                    let dir_entry = dir_entry.ok()?;
                    let path = dir_entry.path();

                    let name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())?;

                    let modified = path.metadata().ok()?.modified().ok()?;

                    let readable = if path.is_file() {
                        File::open(path).is_ok()
                    } else if path.is_dir() {
                        path.read_dir().is_ok()
                    } else {
                        // What else could it be
                        true
                    };

                    cfg_if! {
                        if #[cfg(target_family = "unix")] {
                            let is_hidden = name.starts_with('.');
                        } else {
                            let is_hidden = false;
                        }
                    };

                    Some((
                        dir_entry.path(),
                        DirectoryContentMetadata {
                            readable,
                            modified,
                            name,
                            is_hidden,
                        },
                    ))
                }),
                Err(err) => {
                    tracing::error!(
                        "Ended up in directory we don't have permissions for {:?} due to {}",
                        path,
                        err
                    );
                    self.pathbar_state = PathBarState::Normal(path.parent().unwrap().to_path_buf());
                    self.refresh_current_dir();
                    return;
                }
            };

            self.directory_contents = directory_contents.collect();
            self.sort_contents();
        }
    }
}
