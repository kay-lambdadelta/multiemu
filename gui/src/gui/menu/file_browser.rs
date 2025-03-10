use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};
use strum::EnumIter;

#[derive(PartialEq, Eq, Clone, Copy, Debug, EnumIter)]
pub enum SortingMethod {
    Name,
    Date,
}

#[derive(Clone, Debug)]
pub struct FileBrowserState {
    path: PathBuf,
    directory_contents: Vec<PathBuf>,
    sorting_method: SortingMethod,
}

impl FileBrowserState {
    pub fn new(home_directory: PathBuf) -> Self {
        let mut me = Self {
            path: PathBuf::default(),
            directory_contents: Vec::new(),
            sorting_method: SortingMethod::Name,
        };
        me.change_directory(home_directory);
        me
    }

    pub fn directory(&self) -> &Path {
        &self.path
    }

    pub fn directory_contents(&self) -> &[PathBuf] {
        &self.directory_contents
    }

    pub fn get_sorting_method(&self) -> SortingMethod {
        self.sorting_method
    }

    pub fn set_sorting_method(&mut self, sorting_method: SortingMethod) {
        if self.sorting_method == sorting_method {
            return;
        }

        self.sorting_method = sorting_method;
        self.sort_contents();
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
    }

    pub fn change_directory(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        assert!(path.is_dir());

        self.path = path.clone();
        self.directory_contents = read_dir(path)
            .unwrap()
            .map(|path| path.unwrap().path())
            .collect();

        self.sort_contents();
    }

    pub fn refresh_directory(&mut self) {
        self.change_directory(self.path.clone());
    }
}
