use clap::Subcommand;
use itertools::Itertools;
use multiemu_frontend::environment::Environment;
use multiemu_runtime::program::{PROGRAM_INFORMATION_TABLE, ProgramManager};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use redb::{ReadableDatabase, ReadableMultimapTable};
use regex::Regex;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, LazyLock, RwLock},
};

#[derive(Clone, Debug, Subcommand)]
pub enum NativeAction {
    Import {
        #[clap(required=true, num_args=1..)]
        paths: Vec<PathBuf>,
    },
    FuzzySearch {
        search: String,
        #[clap(short, long, default_value = "0.30")]
        similarity: f64,
    },
}

pub fn database_native_import(
    paths: Vec<PathBuf>,
    environment: Arc<RwLock<Environment>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let environment_guard = environment.read().unwrap();
    let program_manager = ProgramManager::new(
        &environment_guard.database_location,
        &environment_guard.rom_store_directory,
    )
    .unwrap();

    paths
        .into_par_iter()
        .try_for_each(|path| program_manager.load_database(path))?;

    Ok(())
}

fn clean_name(s: &str) -> String {
    static REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\[.*?\]|\(.*?\))").unwrap());

    REGEX.replace_all(s, "").to_string()
}

pub fn database_native_fuzzy_search(
    search: String,
    similarity: f64,
    environment: Arc<RwLock<Environment>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let search = search.to_lowercase();
    let environment_guard = environment.read().unwrap();
    let program_manager = ProgramManager::new(
        &environment_guard.database_location,
        &environment_guard.rom_store_directory,
    )
    .unwrap();

    let database_transaction = program_manager.database().begin_read().unwrap();
    let program_information_table =
        database_transaction.open_multimap_table(PROGRAM_INFORMATION_TABLE)?;

    let mut found_games: HashMap<_, Vec<_>> = HashMap::new();
    for (program_id, name) in program_information_table
        .iter()?
        .filter_map(|entry| {
            entry.ok().map(|(program_id, program_infos)| {
                let program_id = program_id.value();

                program_infos.filter_map(move |rom_info| {
                    rom_info.ok().map(|info| (program_id.clone(), info.value()))
                })
            })
        })
        .flatten()
        .flat_map(|(id, info)| {
            info.names()
                .clone()
                .into_iter()
                .map(move |name| (id.clone(), clean_name(&name)))
        })
    {
        let calculated_similarity = strsim::sorensen_dice(&search, &name);

        if calculated_similarity >= similarity {
            found_games
                .entry(program_id.machine)
                .or_default()
                .push((program_id, calculated_similarity));
        }
    }

    for (machine_id, found_games) in found_games {
        println!("{machine_id}");
        for (game, similarity) in found_games
            .into_iter()
            .sorted_by(|(_, similarity1), (_, similarity2)| similarity1.total_cmp(similarity2))
            .rev()
        {
            println!("\t{similarity:.2} {game}");
        }
    }

    Ok(())
}
