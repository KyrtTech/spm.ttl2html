//! `manifest.json` shape: `routes`, optional `doc_base_url` + `ontology_prefix` for href rewriting.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct PublishConfig {
    pub url: String,
    pub ontology_prefix: String,
    pub should_use_extesion_for_links: bool,
    pub iri_routes: HashMap<String, String>,
    pub index: Option<String>,
}
impl Default for PublishConfig {
    fn default() -> Self {
        PublishConfig {
            url: "".to_string(),
            ontology_prefix: "".to_string(),
            should_use_extesion_for_links: false,
            iri_routes: HashMap::new(),
            index: None,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Manifest {
    pub routes: HashMap<String, String>,
    pub publish: Option<PublishConfig>,
    /// Optional raw HTML injected into every generated page before `</body>`.
    pub html_snippet: Option<String>,
}

impl Manifest {
}
