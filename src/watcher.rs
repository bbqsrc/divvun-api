use std::sync::mpsc::channel;
use std::time::Duration;

use actix::prelude::*;
use log::{error, info};
use notify::Watcher as _;
use notify::{watcher, DebouncedEvent, RecursiveMode};

use crate::file_utils::get_file_info;
use crate::language::data_files::{get_typed_data_dir, DataFileType};
use crate::language::grammar::list_preferences;
use crate::server::state::State;

pub struct Watcher;

impl Actor for Watcher {
    type Context = SyncContext<Self>;
}

pub struct Start {
    pub state: State,
}

impl Message for Start {
    type Result = Result<(), ()>;
}

impl Handler<Start> for Watcher {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: Start, _: &mut Self::Context) -> Self::Result {
        let state = msg.state;
        let data_file_dir = &state.config.data_file_dir;

        let (tx, rx) = channel();

        let interval = state.config.watcher_interval_ms;
        let mut watcher = watcher(tx, Duration::from_millis(interval)).unwrap();

        let dir = get_typed_data_dir(data_file_dir.as_path(), DataFileType::Grammar);
        watcher
            .watch(
                get_typed_data_dir(data_file_dir.as_path(), DataFileType::Grammar),
                RecursiveMode::NonRecursive,
            )
            .unwrap();
        info!("Watching directory `{}` for grammar files", dir.display());

        let dir = get_typed_data_dir(data_file_dir.as_path(), DataFileType::Spelling);
        watcher
            .watch(
                get_typed_data_dir(data_file_dir.as_path(), DataFileType::Spelling),
                RecursiveMode::NonRecursive,
            )
            .unwrap();
        info!("Watching directory `{}` for speller files", dir.display());

        let dir = get_typed_data_dir(data_file_dir.as_path(), DataFileType::Hyphenation);
        watcher
            .watch(
                get_typed_data_dir(data_file_dir.as_path(), DataFileType::Hyphenation),
                RecursiveMode::NonRecursive,
            )
            .unwrap();
        info!(
            "Watching directory `{}` for hyphenation files",
            dir.display()
        );

        loop {
            match rx.recv() {
                Ok(event) => match &event {
                    DebouncedEvent::Create(path) => {
                        info!("Event {:?}", &event);

                        if let Some(file_info) = get_file_info(path) {
                            if file_info.extension == DataFileType::Grammar.as_ext() {
                                let preferences = match list_preferences(file_info.path) {
                                    Ok(preferences) => preferences,
                                    Err(e) => {
                                        error!("Failed to retrieve grammar preferences for {}: {}, ignoring file", e, file_info.stem);
                                        continue;
                                    }
                                };

                                let grammar_checkers =
                                    &state.language_functions.grammar_suggestions;
                                grammar_checkers.add(file_info.stem, file_info.path);

                                let prefs_lock = &mut state.gramcheck_preferences.write();
                                prefs_lock.insert(file_info.stem.to_owned(), preferences);
                            } else if file_info.extension == DataFileType::Spelling.as_ext() {
                                let spellers = &state.language_functions.spelling_suggestions;
                                spellers.add(file_info.stem, file_info.path);
                            } else if file_info.extension == DataFileType::Hyphenation.as_ext() {
                                let hyphenators = &state.language_functions.hyphenation_suggestions;
                                hyphenators.add(file_info.stem, file_info.path);
                            }
                        }
                    }
                    DebouncedEvent::Remove(path) => {
                        info!("Event {:?}", &event);

                        if let Some(file_info) = get_file_info(path) {
                            if file_info.extension == DataFileType::Grammar.as_ext() {
                                let grammar_checkers =
                                    &state.language_functions.grammar_suggestions;
                                grammar_checkers.remove(file_info.stem);

                                let prefs_lock = &mut state.gramcheck_preferences.write();
                                prefs_lock.remove(file_info.stem);
                            } else if file_info.extension == DataFileType::Spelling.as_ext() {
                                let spellers = &state.language_functions.spelling_suggestions;
                                spellers.remove(file_info.stem);
                            } else if file_info.extension == DataFileType::Hyphenation.as_ext() {
                                let hyphenators = &state.language_functions.hyphenation_suggestions;
                                hyphenators.remove(file_info.stem);
                            }
                        }
                    }
                    DebouncedEvent::Write(path) => {
                        info!("Event {:?}", &event);

                        if let Some(file_info) = get_file_info(path) {
                            if file_info.extension == DataFileType::Grammar.as_ext() {
                                let preferences = match list_preferences(file_info.path) {
                                    Ok(preferences) => preferences,
                                    Err(e) => {
                                        error!("Failed to retrieve grammar preferences for {}: {}, ignoring file", e, file_info.stem);
                                        continue;
                                    }
                                };

                                let grammar_checkers =
                                    &state.language_functions.grammar_suggestions;

                                grammar_checkers.remove(file_info.stem);
                                grammar_checkers.add(file_info.stem, file_info.path);

                                let prefs_lock = &mut state.gramcheck_preferences.write();
                                prefs_lock.remove(file_info.stem);
                                prefs_lock.insert(file_info.stem.to_owned(), preferences);
                            } else if file_info.extension == DataFileType::Spelling.as_ext() {
                                let spellers = &state.language_functions.spelling_suggestions;

                                spellers.remove(file_info.stem);
                                spellers.add(file_info.stem, file_info.path);
                            } else if file_info.extension == DataFileType::Hyphenation.as_ext() {
                                let hyphenators = &state.language_functions.hyphenation_suggestions;

                                hyphenators.remove(file_info.stem);
                                hyphenators.add(file_info.stem, file_info.path);
                            }
                        }
                    }
                    _ => info!("Event {:?}", &event),
                },
                Err(e) => error!("Watch error: {:?}", e),
            }
        }
    }
}
