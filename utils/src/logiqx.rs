use codes_iso_639::part_3::LanguageCode;
use codes_iso_3166::part_1::CountryCode;
use multiemu::rom::{ROM_INFORMATION_TABLE, RomId, RomInfo, RomMetadata, System};
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
    pub name: System,
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
                    result.push(substring.trim().to_string());
                }
            }
            _ => {}
        }
    }

    result
}

struct NameMetadataExtractor {
    pub languages: HashSet<LanguageCode>,
    pub regions: HashSet<CountryCode>,
}

impl FromStr for NameMetadataExtractor {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut languages = HashSet::new();
        let mut regions = HashSet::new();

        // Split the string into parts based on parentheses
        let parts = get_data_in_parentheses(s);

        for part in parts {
            let part = part.to_lowercase();
            let part = part.trim().split(',');

            for part in part {
                let part = part.trim();

                if let Ok(language) = codes_iso_639::part_1::LanguageCode::from_str(part) {
                    match language {
                        codes_iso_639::part_1::LanguageCode::En => {
                            languages.insert(LanguageCode::Eng);
                        }
                        codes_iso_639::part_1::LanguageCode::Ja => {
                            languages.insert(LanguageCode::Jpn);
                        }
                        codes_iso_639::part_1::LanguageCode::Fr => {
                            languages.insert(LanguageCode::Fra);
                        }
                        codes_iso_639::part_1::LanguageCode::De => {
                            languages.insert(LanguageCode::Deu);
                        }
                        codes_iso_639::part_1::LanguageCode::Es => {
                            languages.insert(LanguageCode::Spa);
                        }
                        codes_iso_639::part_1::LanguageCode::It => {
                            languages.insert(LanguageCode::Ita);
                        }
                        codes_iso_639::part_1::LanguageCode::Zh => {
                            languages.insert(LanguageCode::Zho);
                        }
                        codes_iso_639::part_1::LanguageCode::Ko => {
                            languages.insert(LanguageCode::Kor);
                        }
                        _ => {}
                    }
                }

                if let Some(region) = match part {
                    "usa" => Some(CountryCode::US),
                    "japan" => Some(CountryCode::JP),
                    // FIXME: Is this what they mean?
                    "europe" => Some(CountryCode::EU),
                    "china" => Some(CountryCode::CN),
                    "korea" => Some(CountryCode::KR),
                    "australia" => Some(CountryCode::AU),
                    "canada" => Some(CountryCode::CA),
                    "united kingdom" => Some(CountryCode::GB),
                    "france" => Some(CountryCode::FR),
                    "brazil" => Some(CountryCode::BR),
                    "italy" => Some(CountryCode::IT),
                    "germany" => Some(CountryCode::DE),
                    "spain" => Some(CountryCode::ES),
                    "taiwan" => Some(CountryCode::TW),
                    _ => None,
                } {
                    regions.insert(region);

                    // No-Intro doesn't seem to mark language when its the """default""" language of the region so we will do this hack
                    if let Some(administrative_language) = region.administrative_language() {
                        languages.insert(administrative_language);
                    }
                }
            }
        }

        Ok(NameMetadataExtractor { languages, regions })
    }
}

pub fn import(
    rom_manager: &RomMetadata,
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

    for machine in data_file.machine {
        let mut dependencies = BTreeSet::default();

        // Extract dependency roms
        for rom in machine.rom {
            let mut languages = HashSet::default();
            let mut regions = HashSet::default();

            if let Ok(name_metadata) = NameMetadataExtractor::from_str(&rom.name) {
                languages.extend(name_metadata.languages);
                regions.extend(name_metadata.regions);
            }

            let info = RomInfo::V0 {
                name: machine.name.clone(),
                path: rom.name.split('\\').map(String::from).collect(),
                system: data_file.header.name,
                languages,
                regions,
            };

            tracing::debug!("Full ROM info: {:#?}", info);

            database_table.insert(rom.id, info)?;
            dependencies.insert(rom.id);
        }
    }

    drop(database_table);
    database_transaction.commit()?;

    Ok(())
}
