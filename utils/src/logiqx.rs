use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    error::Error,
    io::BufRead,
    str::FromStr,
    sync::LazyLock,
};

use fluxemu_locale::{Iso639Alpha2, Iso639Alpha3};
use fluxemu_runtime::program::{
    Filesystem, HASH_ALIAS_TABLE, MachineId, PROGRAM_INFORMATION_TABLE, ProgramId, ProgramInfo,
    ProgramManager, RomId,
};
use serde::{Deserialize, Deserializer};
use serde_with::{DisplayFromStr, serde_as};

#[derive(Debug, Deserialize)]
pub struct Datafile {
    pub header: Header,
    #[serde(alias = "machine")]
    pub game: Vec<Game>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Header {
    #[serde(deserialize_with = "deserialize_nointro_machine_id")]
    #[serde(rename = "name")]
    pub machine_id: MachineId,
}

#[derive(Debug, Deserialize)]
pub struct Game {
    #[serde(rename = "@name")]
    pub name: String,
    pub rom: Vec<Rom>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Rom {
    #[serde(rename = "@name")]
    pub path: String,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "@sha1")]
    pub sha1: RomId,
}

fn get_data_in_parentheses(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut stack = Vec::new();

    for (i, c) in input.char_indices() {
        match c {
            '(' => {
                stack.push(i);
            }
            ')' => {
                if let Some(start) = stack.pop() {
                    let substring = &input[start + 1..i];
                    result.push(substring.trim().to_string());
                }
            }
            _ => {}
        }
    }

    result
}

struct NameMetadataExtractor {
    pub languages: BTreeSet<Iso639Alpha3>,
}

impl FromStr for NameMetadataExtractor {
    type Err = Box<dyn std::error::Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut languages = BTreeSet::new();

        // Split the string into parts based on parentheses
        let parts = get_data_in_parentheses(s);

        for part in parts {
            let part = part.to_lowercase();
            let part = part.trim().split(',');

            for part in part {
                let part = part.trim();

                if let Ok(lang) = Iso639Alpha2::from_str(part) {
                    languages.insert(lang.to_alpha3());
                }

                if let Some(lang) = LANGUAGE_OVERRIDES.get(part) {
                    languages.insert(lang.to_alpha3());
                }
            }
        }

        Ok(NameMetadataExtractor { languages })
    }
}

pub fn import(
    program_manager: &ProgramManager,
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
        data_file.game.len(),
        data_file.header.machine_id
    );

    let database_transaction = program_manager.database().begin_write()?;
    let mut program_information =
        database_transaction.open_multimap_table(PROGRAM_INFORMATION_TABLE)?;
    let mut hash_alias = database_transaction.open_multimap_table(HASH_ALIAS_TABLE)?;

    for game in data_file.game {
        let program_id = ProgramId {
            machine: data_file.header.machine_id,
            name: game.name,
        };

        if !game.rom.is_empty() {
            let first_rom_path: Vec<_> = game.rom[0].path.split('\\').map(String::from).collect();

            // If the rom is a single file like for most early game systems
            let filesystem = if game.rom.len() == 1 && first_rom_path.len() == 1 {
                let rom = &game.rom[0];
                hash_alias.insert(rom.sha1, program_id.clone())?;

                Filesystem::Single {
                    rom_id: rom.sha1,
                    file_name: rom.path.clone(),
                }
            } else {
                let mut filesystem: BTreeMap<_, BTreeSet<_>> = BTreeMap::default();

                for rom in game.rom {
                    hash_alias.insert(rom.sha1, program_id.clone())?;
                    filesystem
                        .entry(rom.sha1)
                        .or_default()
                        .insert(rom.path.replace('\\', "/"));
                }

                Filesystem::Complex(filesystem)
            };

            let name = first_rom_path[0].clone();
            let name_metadata_extractor = NameMetadataExtractor::from_str(&name)?;

            let info = ProgramInfo::V0 {
                names: BTreeSet::from([name]),
                filesystem,
                languages: name_metadata_extractor.languages,
                version: None,
            };

            program_information.insert(program_id.clone(), info)?;
        }
    }

    drop(program_information);
    drop(hash_alias);
    database_transaction.commit()?;

    Ok(())
}

pub fn deserialize_nointro_machine_id<'de, D>(deserializer: D) -> Result<MachineId, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    MachineId::from_nointro_str(&s).map_err(serde::de::Error::custom)
}

static LANGUAGE_OVERRIDES: LazyLock<HashMap<&'static str, Iso639Alpha2>> = LazyLock::new(|| {
    HashMap::from([
        ("usa", Iso639Alpha2::EN),
        ("japan", Iso639Alpha2::JA),
        ("china", Iso639Alpha2::ZH),
        ("korea", Iso639Alpha2::KO),
        ("australia", Iso639Alpha2::EN),
        ("canada", Iso639Alpha2::EN),
        ("united kingdom", Iso639Alpha2::EN),
        ("france", Iso639Alpha2::FR),
        ("brazil", Iso639Alpha2::PT),
        ("italy", Iso639Alpha2::IT),
        ("germany", Iso639Alpha2::DE),
        ("spain", Iso639Alpha2::ES),
        ("taiwan", Iso639Alpha2::ZH),
    ])
});
