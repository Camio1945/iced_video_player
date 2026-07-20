use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DictEntry {
    #[allow(dead_code)]
    pub word: String,
    #[serde(default)]
    pub phonetic: Option<String>,
    #[serde(default)]
    pub phonetics: Vec<DictPhonetic>,
    pub meanings: Vec<DictMeaning>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DictPhonetic {
    #[serde(default)]
    pub text: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    pub audio: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DictMeaning {
    #[serde(rename = "partOfSpeech")]
    pub part_of_speech: String,
    pub definitions: Vec<DictDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DictDefinition {
    pub definition: String,
    #[serde(default)]
    pub example: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct MyMemoryResponse {
    #[serde(rename = "responseData")]
    response_data: MyMemoryData,
    #[allow(dead_code)]
    #[serde(default, rename = "responseStatus")]
    response_status: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
struct MyMemoryData {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

/// Final lookup result that combines Chinese translation and English
/// definitions, ready to be rendered in the side panel.
#[derive(Debug, Clone)]
pub struct DictResult {
    pub word: String,
    /// The Chinese translation (primary) – shown prominently at the top.
    pub chinese: String,
    /// Phonetic / IPA transcription (e.g. `/noʊ/`).
    pub phonetic: String,
    /// English definitions grouped by part of speech.
    pub sections: Vec<DictSection>,
    /// Stand-alone example sentences collected from the definitions.
    pub examples: Vec<String>,
    /// Set if neither the Chinese nor the English lookup returned anything.
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DictSection {
    pub part_of_speech: String,
    /// (definition, optional example)
    pub definitions: Vec<(String, Option<String>)>,
}

/// Look up an English word and return a [`DictResult`] that contains both a
/// Chinese translation (via MyMemory) and English definitions / examples
/// (via dictionaryapi.dev).
pub fn lookup(name: &str) -> DictResult {
    let chinese = lookup_chinese(name);
    let (phonetic, sections, examples) = lookup_english(name);
    let error = if chinese.is_empty() && sections.is_empty() {
        Some(format!("No definition found for \"{}\"", name))
    } else {
        None
    };
    DictResult {
        word: name.to_string(),
        chinese,
        phonetic,
        sections,
        examples,
        error,
    }
}

fn lookup_chinese(word: &str) -> String {
    let encoded: String = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("q", word)
        .append_pair("langpair", "en|zh-CN")
        .finish();
    let url = format!("https://api.mymemory.translated.net/get?{}", encoded);
    match ureq::get(&url).call() {
        Ok(resp) => match resp.into_string() {
            Ok(body) => match serde_json::from_str::<MyMemoryResponse>(&body) {
                Ok(data) => clean_translation(&data.response_data.translated_text),
                Err(_) => String::new(),
            },
            Err(_) => String::new(),
        },
        Err(_) => String::new(),
    }
}

/// The MyMemory API echoes the input wrapped in uppercase messages such as
/// "MYMEMORY WARNING: ..." when the query is empty / out-of-vocabulary.
/// Strip those out so we only show a clean translation.
fn clean_translation(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    if s.contains("MYMEMORY WARNING") || s.contains("PLEASE SELECT TWO DISTINCT") {
        return String::new();
    }
    s.to_string()
}

fn lookup_english(word: &str) -> (String, Vec<DictSection>, Vec<String>) {
    let url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{}", word);
    match ureq::get(&url).call() {
        Ok(resp) => match resp.into_string() {
            Ok(body) => match serde_json::from_str::<Vec<DictEntry>>(&body) {
                Ok(entries) if !entries.is_empty() => {
                    let e = &entries[0];
                    let phonetic = e
                        .phonetic
                        .clone()
                        .or_else(|| e.phonetics.iter().find_map(|p| p.text.clone()))
                        .unwrap_or_default();
                    let mut sections: Vec<DictSection> = Vec::new();
                    let mut examples: Vec<String> = Vec::new();
                    for m in &e.meanings {
                        let mut defs: Vec<(String, Option<String>)> = Vec::new();
                        for d in m.definitions.iter().take(3) {
                            defs.push((d.definition.clone(), d.example.clone()));
                            if let Some(ex) = &d.example {
                                if examples.len() < 5 && !examples.iter().any(|x: &String| x == ex)
                                {
                                    examples.push(ex.clone());
                                }
                            }
                        }
                        sections.push(DictSection {
                            part_of_speech: m.part_of_speech.clone(),
                            definitions: defs,
                        });
                    }
                    (phonetic, sections, examples)
                }
                _ => (String::new(), Vec::new(), Vec::new()),
            },
            Err(_) => (String::new(), Vec::new(), Vec::new()),
        },
        Err(_) => (String::new(), Vec::new(), Vec::new()),
    }
}
