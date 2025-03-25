use super::UiOutput;
use egui::{ScrollArea, Ui};
use multiemu_rom::manager::{ROM_INFORMATION_TABLE, RomManager};
use std::sync::Arc;

const COLUMNS: usize = 5;

#[derive(Debug)]
pub struct DatabaseState {
    rom_manager: Arc<RomManager>,
}

impl DatabaseState {
    pub fn new(rom_manager: Arc<RomManager>) -> Self {
        Self { rom_manager }
    }

    pub fn run(&mut self, output: &mut Option<UiOutput>, ui: &mut Ui) {
        let read_transaction = self.rom_manager.rom_information.begin_read().unwrap();
        let table = read_transaction.open_table(ROM_INFORMATION_TABLE).unwrap();
        let loaded_roms_guard = self.rom_manager.loaded_roms.read().unwrap();

        ScrollArea::vertical().show_rows(ui, 1.0, loaded_roms_guard.len(), |ui, rows| {
            for row in rows {
                let (rom_id, rom_location) = loaded_roms_guard.get_index(row).unwrap();
                let rom_info = table.get(rom_id).unwrap().unwrap().value();

                if ui.button(rom_info.name).clicked() {
                    *output = Some(UiOutput::OpenGame { rom_id: *rom_id });
                }
            }
        });
    }
}
