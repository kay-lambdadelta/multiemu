use isolang::Language;
use multiemu_rom::{
    id::RomId,
    info::RomInfoV0,
    manager::{ROM_INFORMATION_TABLE, RomManager},
    system::GameSystem,
};
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use std::{
    collections::{BTreeSet, HashSet},
    error::Error,
    io::BufRead,
    str::FromStr,
};

#[derive(Debug, Deserialize)]
pub struct Datafile {
    pub header: Header,
    #[serde(alias = "game")]
    pub machine: Vec<Machine>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Header {
    #[serde_as(as = "DisplayFromStr")]
    pub name: GameSystem,
}

#[derive(Debug, Deserialize)]
pub struct Machine {
    #[serde(rename = "@name")]
    pub name: String,
    pub rom: Vec<Rom>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Rom {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "@sha1")]
    pub id: RomId,
}

fn get_data_in_parentheses(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut stack = Vec::new();

    for (i, c) in input.chars().enumerate() {
        match c {
            '(' => {
                stack.push(i);
            }
            ')' => {
                if let Some(start) = stack.pop() {
                    let substring = &input[start + 1..i];
                    result.push(substring.to_string());
                }
            }
            _ => {}
        }
    }

    result
}

struct NameMetadataExtractor {
    pub languages: HashSet<Language>,
}

impl FromStr for NameMetadataExtractor {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut languages = HashSet::new();

        // Split the string into parts based on parentheses
        let parts = get_data_in_parentheses(s);

        for part in parts {
            let part = part.to_lowercase();
            let part = part.trim().split(',');

            for part in part {
                if let Some(language) = Language::from_639_1(part) {
                    languages.insert(language);
                }

                // Region to default locale
                match part {
                    "usa" => {
                        languages.insert(Language::from_639_1("en").unwrap());
                    }
                    "united kingdom" => {
                        languages.insert(Language::from_639_1("en").unwrap());
                    }
                    "japan" => {
                        languages.insert(Language::from_639_1("ja").unwrap());
                    }
                    _ => {}
                }
            }
        }

        Ok(NameMetadataExtractor { languages })
    }
}

pub fn import(
    rom_manager: &RomManager,
    file: impl BufRead,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Parse XML based data file
    let data_file: Datafile = match quick_xml::de::from_reader(file) {
        Ok(file) => file,
        Err(err) => {
            tracing::error!("Failed to parse XML logiqx database: {}", err);
            return Ok(());
        }
    };

    tracing::info!(
        "Found {} entries in XML logiqx database for the system {}",
        data_file.machine.len(),
        data_file.header.name
    );

    let database_transaction = rom_manager.rom_information.begin_write()?;
    let mut database_table = database_transaction.open_multimap_table(ROM_INFORMATION_TABLE)?;

    for mut machine in data_file.machine {
        let mut dependencies = BTreeSet::default();

        // Extract dependency roms
        for rom in machine.rom.drain(1..) {
            let mut languages = BTreeSet::default();

            if let Ok(name_metadata) = NameMetadataExtractor::from_str(&rom.name) {
                languages.extend(name_metadata.languages);
            }

            let info = RomInfoV0 {
                name: machine.name.clone(),
                file_name: rom.name.replace('\\', "/").into(),
                system: data_file.header.name,
                languages,
                dependencies: BTreeSet::default(),
            };

            tracing::debug!("Full ROM info: {:#?}", info);

            database_table.insert(rom.id, info)?;
            dependencies.insert(rom.id);
        }

        let rom = machine.rom.remove(0);
        let mut languages = BTreeSet::default();

        if let Ok(name_metadata) = NameMetadataExtractor::from_str(&rom.name) {
            languages.extend(name_metadata.languages);
        }

        let info = RomInfoV0 {
            name: machine.name,
            file_name: rom.name.replace('\\', "/").into(),
            system: data_file.header.name,
            languages,
            dependencies,
        };

        tracing::debug!("Full ROM info: {:#?}", info);

        database_table.insert(rom.id, info)?;
    }

    drop(database_table);
    database_transaction.commit()?;

    Ok(())
}
