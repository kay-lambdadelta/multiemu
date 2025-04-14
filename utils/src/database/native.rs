use clap::Subcommand;
use multiemu_config::Environment;
use multiemu_rom::manager::{ROM_INFORMATION_TABLE, RomManager};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use redb::ReadableMultimapTable;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

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

pub fn database_native_import(
    paths: Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let environment = Environment::load()?;
    let rom_manager = Arc::new(
        RomManager::new(
            Some(&environment.database_file),
            Some(&environment.roms_directory),
        )
        .unwrap(),
    );
    paths
        .into_par_iter()
        .try_for_each(|path| rom_manager.load_database(path))?;

    Ok(())
}

pub fn database_native_fuzzy_search(
    search: String,
    similarity: f64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let search = search.to_lowercase();
    let environment = Environment::load()?;
    let rom_manager = Arc::new(
        RomManager::new(
            Some(&environment.database_file),
            Some(&environment.roms_directory),
        )
        .unwrap(),
    );
    let database_transaction = rom_manager.rom_information.begin_read().unwrap();
    let database_table = database_transaction.open_multimap_table(ROM_INFORMATION_TABLE)?;

    let mut found_games: HashMap<_, Vec<_>> = HashMap::new();
    for rom_info in database_table
        .iter()?
        .filter_map(|entry| {
            entry.ok().map(|(_, rom_infos)| {
                rom_infos.filter_map(|rom_info| rom_info.ok().map(|info| info.value()))
            })
        })
        .flatten()
    {
        let calculated_similarity =
            strsim::jaro_winkler(&search, &rom_info.file_name.to_string().to_lowercase());

        if calculated_similarity >= similarity {
            found_games
                .entry(rom_info.system)
                .or_default()
                .push((rom_info.file_name.clone(), calculated_similarity));
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
