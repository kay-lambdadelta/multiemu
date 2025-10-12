use crate::gui::UiOutput;
use std::time::{Duration, Instant};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use versions::SemVer;

#[derive(Debug)]
pub struct Properties {
    pub multiemu_version: Option<SemVer>,
    pub os_name: Option<String>,
    pub max_memory: u64,
}

#[derive(Debug)]
pub struct AboutState {
    properties: Option<Properties>,
    properties_last_updated: Instant,

    // System information
    system: System,
}

impl Default for AboutState {
    fn default() -> Self {
        Self {
            properties: None,
            properties_last_updated: Instant::now(),
            system: System::new(),
        }
    }
}

impl AboutState {
    pub fn run(&mut self, output: &mut Option<UiOutput>, ui: &mut egui::Ui) {
        // We do not want to update this per frame
        if self.properties.is_none()
            || self.properties_last_updated.elapsed() > Duration::from_secs(1)
        {
            self.properties.take();

            self.system.refresh_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                    .with_memory(MemoryRefreshKind::nothing().with_ram().with_swap()),
            );

            let properties = Properties {
                multiemu_version: option_env!("CARGO_PKG_VERSION")
                    .map(|version| version.parse().unwrap()),
                os_name: System::long_os_version().or_else(System::name),
                max_memory: self.system.total_memory(),
            };

            self.properties = Some(properties);
            self.properties_last_updated = Instant::now();
        }

        let properties = self.properties.as_ref().unwrap();

        ui.vertical(|ui| {
            if let Some(version) = &properties.multiemu_version {
                ui.label("MultiEMU Version");
                ui.separator();
                ui.label(version.to_string());
            }
        });
    }
}
