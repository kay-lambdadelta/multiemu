use super::UiOutput;
use egui_extras::{Column, TableBuilder};
use multiemu_machine::Machine;

const ROW_WIDTH: usize = 8;

#[derive(Debug, Default)]
pub struct MemoryViewerState {}

impl MemoryViewerState {
    pub fn run(
        &mut self,
        output: &mut Option<UiOutput>,
        ui: &mut egui::Ui,
        machine: Option<&Machine>,
    ) {
        if let Some(machine) = machine {
            for address_space in machine.memory_translation_table.address_spaces() {
                let address_space_width = machine
                    .memory_translation_table
                    .get_address_space_width(address_space)
                    .unwrap();
                let max_number = 2usize.pow(address_space_width as u32) - 1;

                TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::auto())
                    .columns(Column::auto(), ROW_WIDTH)
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.label("Address");
                        });
                        for i in 0..ROW_WIDTH {
                            header.col(|ui| {
                                ui.label(format!("{:02x}", i));
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(1.0, max_number, |mut row| {
                            let address = row.index() * ROW_WIDTH;
                            row.col(|ui| {
                                ui.label(format!("{:04x}", address));
                            });
                            for i in 0..ROW_WIDTH {
                                row.col(|ui| {
                                    let value = machine
                                        .memory_translation_table
                                        .preview_le_value::<u8>(address + i, address_space)
                                        .unwrap();
                                    ui.label(format!("{:02x}", value));
                                });
                            }
                        });
                    });
            }
        }
    }
}
