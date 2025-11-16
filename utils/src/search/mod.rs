use std::sync::LazyLock;

use clap::Parser;
use itertools::Itertools;
use multiemu_frontend::environment::Environment;
use multiemu_runtime::program::{PROGRAM_INFORMATION_TABLE, ProgramManager};
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use redb::{ReadableDatabase, ReadableMultimapTable};
use regex::Regex;

#[derive(Clone, Parser)]
pub enum SearchAction {
    Fuzzy {
        search: String,
        #[clap(short, long, default_value = "0.30")]
        similarity: f64,
    },
    Exact {
        search: String,
    },
    Regex {
        regex: Regex,
    },
}

pub fn search(
    environment: Environment,
    search_action: SearchAction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let program_manager = ProgramManager::new(
        &environment.database_location,
        &environment.rom_store_directory,
    )
    .unwrap();

    let database_transaction = program_manager.database().begin_read().unwrap();
    let program_information_table =
        database_transaction.open_multimap_table(PROGRAM_INFORMATION_TABLE)?;

    let found_games: scc::HashMap<_, Vec<_>> = scc::HashMap::new();
    program_information_table
        .iter()?
        .par_bridge()
        .filter_map(|entry| {
            entry.ok().map(|(program_id, program_infos)| {
                let program_id = program_id.value();

                program_infos.par_bridge().filter_map(move |rom_info| {
                    rom_info.ok().map(|info| (program_id.clone(), info.value()))
                })
            })
        })
        .flatten()
        .flat_map(|(id, info)| {
            info.names()
                .clone()
                .into_par_iter()
                .map(move |name| (id.clone(), clean_name(&name)))
        })
        .for_each(|(program_id, name)| match &search_action {
            SearchAction::Fuzzy { search, similarity } => {
                let calculated_similarity = strsim::sorensen_dice(search, &name);

                if calculated_similarity >= *similarity {
                    found_games
                        .entry_sync(program_id.machine)
                        .or_default()
                        .push((program_id, calculated_similarity));
                }
            }
            SearchAction::Exact { search } => {
                if name.contains(search) {
                    found_games
                        .entry_sync(program_id.machine)
                        .or_default()
                        .push((program_id, 1.0));
                }
            }
            SearchAction::Regex { regex } => {
                if regex.is_match(&name) {
                    found_games
                        .entry_sync(program_id.machine)
                        .or_default()
                        .push((program_id, 1.0));
                }
            }
        });

    found_games.iter_sync(|machine_id, found_games| {
        println!("{machine_id}");
        for (game, similarity) in found_games
            .iter()
            .sorted_by(|(_, similarity1), (_, similarity2)| similarity1.total_cmp(similarity2))
            .rev()
        {
            println!("\t{similarity:.2} {game}");
        }

        true
    });

    Ok(())
}

fn clean_name(s: &str) -> String {
    static REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\[.*?\]|\(.*?\))").unwrap());

    REGEX.replace_all(s, "").to_string()
}
