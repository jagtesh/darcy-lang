use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Defs {
    pub language_id: String,
    pub scope: String,
    pub file_extensions: Vec<String>,
    pub keywords: Vec<String>,
    pub types: Vec<String>,
    pub builtins: Vec<String>,
    pub operators: Vec<String>,
    pub comment_line: String,
    pub comment_block: (String, String),
}

impl Defs {
    pub fn load() -> Self {
        let raw = include_str!("../../../extensions/defs.json");
        serde_json::from_str(raw).expect("defs.json is valid")
    }
}
