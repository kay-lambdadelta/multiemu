use clap::Subcommand;
use multiemu_config::Environment;
use multiemu_rom::manager::{ROM_INFORMATION_TABLE, RomManager};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use redb::ReadableTable;
use std::collections::HashMap;
use std::{error::Error, path::PathBuf};

#[derive(Clone, Debug, Subcommand)]
pub enum NativeAction {
    Import {
        #[clap(required=true, num_args=1..)]
        paths: Vec<PathBuf>,
    },
    FuzzySearch {
        search: String,
        #[clap(short, long, default_value = "0.80")]
        similarity: f64,
    },
}

pub fn database_native_import(paths: Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    let environment = Environment::load()?;
    let rom_manager = RomManager::new(Some(&environment.database_file))?;

    paths
        .into_par_iter()
        .try_for_each(|path| rom_manager.load_database(path))
        .map_err(|err| err as Box<dyn Error>)?;

    Ok(())
}

pub fn database_native_fuzzy_search(search: String, similarity: f64) -> Result<(), Box<dyn Error>> {
    let search = search.to_lowercase();
    let environment = Environment::load()?;
    let rom_manager = RomManager::new(Some(&environment.database_file))?;
    let database_transaction = rom_manager.rom_information.begin_read().unwrap();
    let database_table = database_transaction.open_table(ROM_INFORMATION_TABLE)?;

    let mut found_games: HashMap<_, Vec<_>> = HashMap::new();

    for rom_info in database_table
        .iter()?
        .filter_map(|rom_info| rom_info.ok().map(|(_, rom_info)| rom_info.value()))
    {
        let calculated_similarity = strsim::jaro_winkler(&search, &rom_info.name.to_lowercase());

        if calculated_similarity >= similarity {
            found_games
                .entry(rom_info.system)
                .or_default()
                .push((rom_info.name.clone(), calculated_similarity));
        }
    }

    for (system, found_games) in found_games {
        println!("{}", system);
        for (game, similarity) in found_games {
            println!("\t{:.2} {}", similarity, game);
        }
    }

    Ok(())
}
