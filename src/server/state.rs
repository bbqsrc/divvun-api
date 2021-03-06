use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use actix::Addr;
use futures::future::{err, ok, Future};
use hashbrown::HashMap;
use parking_lot::RwLock;

use crate::config::Config;
use crate::error::ApiError;
use crate::file_utils::get_file_info;
use crate::graphql::schema::create_schema;
use crate::graphql::schema::Schema;
use crate::language::data_files::{get_data_files, DataFileType};
use crate::language::grammar::{
    list_preferences, AsyncGramchecker, GramcheckExecutor, GramcheckRequest, GramcheckResponse,
};
use crate::language::hyphenation::{
    AsyncHyphenator, HyphenationExecutor, HyphenationRequest, HyphenationResponse,
};
use crate::language::speller::{
    AsyncSpeller, DivvunSpellExecutor, SpellerRequest, SpellerResponse,
};

pub struct LanguageFunctions {
    pub spelling_suggestions:
        Box<dyn LanguageSuggestions<Request = SpellerRequest, Response = SpellerResponse>>,
    pub grammar_suggestions:
        Box<dyn LanguageSuggestions<Request = GramcheckRequest, Response = GramcheckResponse>>,
    pub hyphenation_suggestions:
        Box<dyn LanguageSuggestions<Request = HyphenationRequest, Response = HyphenationResponse>>,
}

pub trait LanguageSuggestions: Send + Sync {
    type Request;
    type Response;

    fn suggestions(
        &self,
        message: Self::Request,
        language: &str,
    ) -> Box<dyn Future<Item = Self::Response, Error = ApiError>>;
    fn add(&self, language: &str, path: &str) -> Box<dyn Future<Item = (), Error = ApiError>>;
    fn remove(&self, language: &str) -> Box<dyn Future<Item = (), Error = ApiError>>;
}

pub trait UnhoistFutureExt<U, E> {
    fn unhoist(self) -> Box<dyn Future<Item = U, Error = E>>;
}

impl<T: 'static, U: 'static, E: 'static> UnhoistFutureExt<U, E> for T
where
    T: Future<Item = Result<U, E>, Error = E>,
{
    fn unhoist(self) -> Box<dyn Future<Item = U, Error = E>> {
        Box::new(self.and_then(|res| match res {
            Ok(result) => ok(result),
            Err(e) => err(e),
        }))
    }
}

pub type State = Arc<InnerState>;

pub struct InnerState {
    pub config: Config,
    pub graphql_schema: Schema,
    pub language_functions: LanguageFunctions,
    pub gramcheck_preferences: Arc<RwLock<HashMap<String, BTreeMap<String, String>>>>,
}

pub fn create_state(config: &Config) -> State {
    let grammar_data_files = get_data_files(config.data_file_dir.as_path(), DataFileType::Grammar)
        .unwrap_or_else(|e| {
            log::error!("Error getting grammar data files: {}", e);
            vec![]
        });

    Arc::new(InnerState {
        config: config.clone(),
        graphql_schema: create_schema(),
        language_functions: LanguageFunctions {
            spelling_suggestions: Box::new(get_speller(config)),
            grammar_suggestions: Box::new(get_gramchecker(&grammar_data_files)),
            hyphenation_suggestions: Box::new(get_hyphenation(config)),
        },
        gramcheck_preferences: Arc::new(RwLock::new(get_gramcheck_preferences(
            &grammar_data_files,
        ))),
    })
}

fn get_speller(config: &Config) -> AsyncSpeller {
    let spelling_data_files =
        get_data_files(config.data_file_dir.as_path(), DataFileType::Spelling).unwrap_or_else(
            |e| {
                log::error!("Error getting spelling data files: {}", e);
                vec![]
            },
        );

    let speller = AsyncSpeller {
        spellers: Arc::new(RwLock::new(
            HashMap::<String, Addr<DivvunSpellExecutor>>::new(),
        )),
    };

    for file in spelling_data_files {
        if let Some(file_info) = get_file_info(&file) {
            speller.add(file_info.stem, file_info.path);
        }
    }

    speller
}

fn get_gramchecker(grammar_data_files: &Vec<PathBuf>) -> AsyncGramchecker {
    let gramchecker = AsyncGramchecker {
        gramcheckers: Arc::new(RwLock::new(
            HashMap::<String, Addr<GramcheckExecutor>>::new(),
        )),
    };

    for file in grammar_data_files {
        if let Some(file_info) = get_file_info(&file) {
            gramchecker.add(file_info.stem, file_info.path);
        }
    }

    gramchecker
}

fn get_hyphenation(config: &Config) -> AsyncHyphenator {
    let hyphenation_data_files =
        get_data_files(config.data_file_dir.as_path(), DataFileType::Hyphenation).unwrap_or_else(
            |e| {
                log::error!("Error getting hyphenation data files: {}", e);
                vec![]
            },
        );

    let hyphenator = AsyncHyphenator {
        hyphenators: Arc::new(RwLock::new(
            HashMap::<String, Addr<HyphenationExecutor>>::new(),
        )),
    };

    for file in hyphenation_data_files {
        if let Some(file_info) = get_file_info(&file) {
            hyphenator.add(file_info.stem, file_info.path);
        }
    }

    hyphenator
}

fn get_gramcheck_preferences(
    grammar_data_files: &Vec<PathBuf>,
) -> HashMap<String, BTreeMap<String, String>> {
    let gramcheck_preferences = grammar_data_files
        .into_iter()
        .map(|f| {
            let grammar_checker_path = f.to_str().unwrap();
            let lang_code = f.file_stem().unwrap().to_str().unwrap();
            (
                lang_code.into(),
                list_preferences(grammar_checker_path).unwrap(),
            )
        })
        .collect();

    gramcheck_preferences
}
