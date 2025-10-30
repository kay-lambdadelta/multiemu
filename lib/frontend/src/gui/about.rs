use crate::gui::UiOutput;
use byte_unit::{Byte, UnitType};
use egui::Ui;
use egui_extras::{Column, TableBuilder};
use std::time::{Duration, Instant};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use versions::SemVer;

#[derive(Debug)]
pub struct Properties {
    pub multiemu_version: Option<SemVer>,
    pub os_name: Option<String>,
    pub max_memory: Byte,
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
    pub fn run(&mut self, _output: &mut Option<UiOutput>, ui: &mut Ui) {
        // Refresh properties every second
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
                max_memory: self.system.total_memory().into(),
            };

            self.properties = Some(properties);
            self.properties_last_updated = Instant::now();
        }

        let properties = self.properties.as_ref().unwrap();
        let appropiate_unit = properties.max_memory.get_appropriate_unit(UnitType::Binary);

        // Prepare the table data
        let rows = vec![
            (
                "MultiEMU Version",
                properties
                    .multiemu_version
                    .as_ref()
                    .map(std::string::ToString::to_string),
            ),
            ("OS Identifier", properties.os_name.clone()),
            ("Max Memory", Some(format!("{appropiate_unit:.2}"))),
        ];

        // Build the table
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::auto().at_least(150.0))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Property");
                });
                header.col(|ui| {
                    ui.label("Value");
                });
            })
            .body(|mut body| {
                for (label, value) in rows {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label(label);
                        });
                        row.col(|ui| {
                            ui.label(value.unwrap_or_else(|| "Unknown".to_string()));
                        });
                    });
                }
            });
    }
}
