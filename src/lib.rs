use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_extern_fn,
    sabi_trait::prelude::TD_Opaque,
    std_types::{RBox, RStr, RString, RVec},
};
use quick_search_lib::{ColoredChar, PluginId, SearchLib, SearchLib_Ref, SearchResult, Searchable, Searchable_TO};

static NAME: &str = "English Dictionary";

#[export_root_module]
pub fn get_library() -> SearchLib_Ref {
    SearchLib { get_searchable }.leak_into_prefix()
}

#[sabi_extern_fn]
fn get_searchable(id: PluginId) -> Searchable_TO<'static, RBox<()>> {
    let this = EnglishDictionary::new(id);
    Searchable_TO::from_value(this, TD_Opaque)
}

#[derive(Debug, Clone)]
struct EnglishDictionary {
    id: PluginId,
    client: reqwest::blocking::Client,
}

impl EnglishDictionary {
    fn new(id: PluginId) -> Self {
        Self {
            id,
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl Searchable for EnglishDictionary {
    fn search(&self, query: RString) -> RVec<SearchResult> {
        let mut res: Vec<SearchResult> = vec![];

        match DictionaryApiResponse::get_word(&query, &self.client) {
            Ok(words) => {
                for word in words {
                    for meaning in word.meanings {
                        for definition in meaning.definitions {
                            let pos = meaning.part_of_speech.to_string();
                            let resstr = format!("{}: {}", pos, definition.definition);
                            let clipboard = format!(
                                "{}. {}: {}{}",
                                word.word,
                                pos,
                                definition.definition,
                                definition.example.as_ref().map(|s| format!("\n{}", s)).unwrap_or_default()
                            );

                            res.push(SearchResult::new(&resstr).set_context(&definition.example.unwrap_or_default()).set_extra_info(&clipboard))
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("failed to get word: {}", e);
            }
        }

        // res.sort_by(|a, b| a.title().cmp(b.title()));
        // res.dedup_by(|a, b| a.title() == b.title());

        res.into()
    }
    fn name(&self) -> RStr<'static> {
        NAME.into()
    }
    fn colored_name(&self) -> RVec<quick_search_lib::ColoredChar> {
        // can be dynamic although it's iffy how it might be used
        // ColoredChar::from_string(NAME, 0x16BE2FFF)
        ColoredChar::from_string(NAME, 0x00FF00FF)
    }
    fn execute(&self, result: &SearchResult) {
        let s = result.extra_info();
        if let Ok::<clipboard::ClipboardContext, Box<dyn std::error::Error>>(mut clipboard) = clipboard::ClipboardProvider::new() {
            if let Ok(()) = clipboard::ClipboardProvider::set_contents(&mut clipboard, s.to_owned()) {
                log::trace!("copied to clipboard: {}", s);
            } else {
                log::error!("failed to copy to clipboard: {}", s);
            }
        } else {
            log::error!("failed to copy to clipboard: {}", s);
        }

        // finish up, above is a clipboard example
    }
    fn plugin_id(&self) -> &PluginId {
        &self.id
    }
}

#[derive(serde::Deserialize, Debug)]
struct DictionaryApiResponse {
    word: String,
    // phonetic: String,
    // phonetics: Vec<Phonetic>,
    meanings: Vec<Meaning>,
    // license: License,
    // #[serde(rename = "sourceUrls")]
    // source_urls: Vec<String>,
}

// #[derive(serde::Deserialize, Debug)]
// struct Phonetic {
//     text: String,
//     audio: String,
//     #[serde(rename = "sourceUrl")]
//     source_url: Option<String>,
//     license: Option<License>,
// }

#[derive(serde::Deserialize, Debug, Clone)]
struct Meaning {
    #[serde(rename = "partOfSpeech")]
    // part_of_speech: PartOfSpeech,
    part_of_speech: String,
    definitions: Vec<WordDefinition>,
    // synonyms: Vec<String>,
    // antonyms: Vec<String>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct WordDefinition {
    definition: String,
    example: Option<String>,
    // synonyms: Vec<String>,
    // antonyms: Vec<String>,
}

// #[derive(serde::Deserialize, Debug)]
// struct License {
//     name: String,
//     url: String,
// }

// #[derive(serde::Deserialize, Debug, Clone)]
// enum PartOfSpeech {
//     #[serde(rename = "noun")]
//     Noun,
//     #[serde(rename = "verb")]
//     Verb,
//     #[serde(rename = "adjective")]
//     Adjective,
//     #[serde(rename = "adverb")]
//     Adverb,
//     #[serde(rename = "pronoun")]
//     Pronoun,
//     #[serde(rename = "preposition")]
//     Preposition,
//     #[serde(rename = "conjunction")]
//     Conjunction,
//     #[serde(rename = "determiner")]
//     Determiner,
//     #[serde(rename = "exclamation")]
//     Exclamation,
//     #[serde(rename = "interjection")]
//     Interjection,
// }

// impl std::fmt::Display for PartOfSpeech {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             PartOfSpeech::Noun => write!(f, "noun"),
//             PartOfSpeech::Verb => write!(f, "verb"),
//             PartOfSpeech::Adjective => write!(f, "adjective"),
//             PartOfSpeech::Adverb => write!(f, "adverb"),
//             PartOfSpeech::Pronoun => write!(f, "pronoun"),
//             PartOfSpeech::Preposition => write!(f, "preposition"),
//             PartOfSpeech::Conjunction => write!(f, "conjunction"),
//             PartOfSpeech::Determiner => write!(f, "determiner"),
//             PartOfSpeech::Exclamation => write!(f, "exclamation"),
//         }
//     }
// }

impl DictionaryApiResponse {
    fn get_word(word: &str, client: &reqwest::blocking::Client) -> anyhow::Result<Vec<Self>> {
        let url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{}", urlencoding::encode(word));

        let response = client.get(url).send()?;

        let json = response.json()?;

        Ok(json)
    }
}
