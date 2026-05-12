use std::collections::HashMap;
use std::fs;

use std::path::{Path, PathBuf};

use rio_api::model::{Literal, NamedNode, Subject, Term};
use rio_api::parser::TriplesParser;
use rio_turtle::{TurtleError, TurtleParser};

use tera::{Context, Tera};

use serde::Serialize;
use url::Url;

use crate::manifest::PublishConfig;

//  well known prefixes that will be used to link to the corresponding HTML pages
const PREFIXES: [&str; 2] = [
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "http://www.w3.org/2000/01/rdf-schema#",
];

#[derive(Serialize, Debug)]
pub struct Triple {
    subject: String,
    predicate: String,
    object: String,

    subject_link: Option<String>,
    subject_label: String,
    predicate_link: Option<String>,
    should_predicate_link_open_in_new_tab: bool,
    object_link: Option<String>,
    should_object_link_open_in_new_tab: bool,
}

impl Default for Triple {
    fn default() -> Self {
        Triple {
            subject: String::new(),
            predicate: String::new(),
            object: String::new(),
            predicate_link: None,
            subject_link: None,
            subject_label: String::new(),
            object_link: None,
            should_predicate_link_open_in_new_tab: false,
            should_object_link_open_in_new_tab: false,
        }
    }
}

#[derive(Serialize)]
pub struct SubjectGroup {
    subject: String,
    subject_label: String,
    /// HTML fragment from entity name (IRI fragment / path tail / blank id), not a hash.
    subject_anchor: String,
    subject_link: Option<String>,
    triples: Vec<Triple>,
}

#[derive(Serialize)]
pub struct IndexEntry {
    path: String,
    name: String,
}

impl IndexEntry {
    pub fn new(path: String, name: String) -> Self {
        IndexEntry { name, path }
    }
}

fn rewrite_iri(iri: &str, publish_config: &Option<PublishConfig>) -> String {
    if let Some(publish_config) = publish_config {
        if iri.starts_with(&publish_config.ontology_prefix) {
            let new_iri = iri.replace(&publish_config.ontology_prefix, &publish_config.url);
            let new_iri_url = Url::parse(&new_iri);

            if let Ok(mut new_iri_url) = new_iri_url {
                let path = &new_iri_url.path()[1..];

                if let Some(rewrite) = publish_config.iri_routes.get(path) {
                    new_iri_url.set_path(rewrite);
                }

                if publish_config.should_use_extesion_for_links {
                    let mut path = new_iri_url.path().to_string();
                    path.push_str(".html");
                    new_iri_url.set_path(&path);
                    return new_iri_url.to_string();
                } else {
                    return new_iri_url.to_string();
                }
            }
            return iri.replace(&publish_config.ontology_prefix, &publish_config.url);
        }
    }
    iri.to_string()
}

/// Absolute `http:` / `https:` links are external unless they start with `publish.url` (trimmed, no trailing slash).
fn href_is_external(href: &str, publish_config: &Option<PublishConfig>) -> bool {
    if !href.starts_with("http://") && !href.starts_with("https://") {
        return false;
    }
    let Some(p) = publish_config else {
        return true;
    };
    let base = p.url.trim().trim_end_matches('/');
    if base.is_empty() {
        return true;
    }
    !href.starts_with(base)
}

fn apply_new_tab_flags(triple: &mut Triple, publish_config: &Option<PublishConfig>) {
    if let Some(predicate_link) = triple.predicate_link.as_ref() {
        triple.should_predicate_link_open_in_new_tab =
            href_is_external(predicate_link, publish_config);
    }
    if let Some(object_link) = triple.object_link.as_ref() {
        triple.should_object_link_open_in_new_tab = href_is_external(object_link, publish_config);
    }
}

fn update_triple_with_links(
    triple: &mut Triple,
    prefixes: &[String],
    publish_config: &Option<PublishConfig>,
) {
    if is_valid_url(&triple.subject) {
        for prefix in prefixes {
            if triple.subject.starts_with(prefix) {
                triple.subject_link = Some(triple.subject.clone());
                triple.subject_label = triple.subject.replace(prefix, "");

                break;
            }
        }
    }

    if is_valid_url(&triple.predicate) {
        for prefix in prefixes {
            if triple.predicate.starts_with(prefix) {
                triple.predicate_link = Some(triple.predicate.clone());
                triple.predicate = triple.predicate.replace(prefix, "");

                if let Some(subject_link) = triple.predicate_link.as_ref() {
                    triple.predicate_link = Some(rewrite_iri(subject_link, publish_config));
                }

                break;
            }
        }
    }

    if is_valid_url(&triple.object) {
        let mut was_prefix_found = false;

        for prefix in prefixes {
            if triple.object.starts_with(prefix) {
                triple.object_link = Some(triple.object.clone());
                triple.object = triple.object.replace(prefix, "");

                if let Some(object_link) = triple.object_link.as_ref() {
                    triple.object_link = Some(rewrite_iri(object_link, publish_config));
                }

                was_prefix_found = true;
                break;
            }
        }

        if !was_prefix_found {
            // we may still have a valid url, but not a prefix
            // this could be an external link
            triple.object_link = Some(triple.object.clone());
        }
    }
}

pub fn convert_file(
    input_path: &Path,
    output_path: &Path,
    index_path: &str,
    tera: &Tera,
    publish_config: &Option<PublishConfig>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let input = fs::read_to_string(input_path)?;
    let mut triples = Vec::new();

    let mut parser = TurtleParser::new(input.as_bytes(), None);

    loop {
        let mut unparsed_triples = Vec::new();

        let _ = parser.parse_step(&mut |t| {
            let subject = match t.subject {
                Subject::NamedNode(NamedNode { iri }) => iri.to_string(),
                Subject::BlankNode(blank) => blank.to_string(),
                Subject::Triple(_) => String::new(),
            };

            let predicate = t.predicate.iri.to_string();
            let object = match t.object {
                Term::NamedNode(NamedNode { iri }) => iri.to_string(),
                Term::Literal(Literal::Simple { value }) => {
                    value.to_string().trim_matches('"').to_string()
                }
                Term::Literal(Literal::Typed { value, datatype: _ }) => {
                    format!("{}", value)
                }
                Term::Literal(Literal::LanguageTaggedString { value, language }) => {
                    format!("{} (@{})", value, language)
                }
                _ => String::new(),
            };

            let triple = Triple {
                subject_label: subject.clone(),
                subject,
                predicate,
                object,
                subject_link: None,
                predicate_link: None,
                object_link: None,
                should_predicate_link_open_in_new_tab: false,
                should_object_link_open_in_new_tab: false,
            };

            unparsed_triples.push(triple);

            Ok::<(), TurtleError>(())
        });

        let mut prefixes: Vec<String> = parser.prefixes().values().cloned().collect();
        prefixes.extend(PREFIXES.iter().map(|p| p.to_string()));

        for mut triple in unparsed_triples {
            update_triple_with_links(&mut triple, &prefixes, publish_config);
            apply_new_tab_flags(&mut triple, publish_config);
            triples.push(triple);
        }

        if parser.is_end() {
            break;
        }
    }

    let mut subject_groups_map = HashMap::new();
    for triple in triples {
        subject_groups_map
            .entry(triple.subject.clone())
            .or_insert_with(Vec::new)
            .push(triple)
    }

    let mut subject_groups: Vec<SubjectGroup> = subject_groups_map
        .into_iter()
        .map(|(subject, mut triples)| {
            //  sort by predicate
            triples.sort_by(|a, b| a.predicate.cmp(&b.predicate));

            let subject_label = triples[0].subject_label.clone();
            SubjectGroup {
                subject_anchor: anchor_for_subject(&subject, &subject_label),
                subject,
                subject_link: triples[0].subject_link.clone(),
                subject_label: subject_label.to_owned(),
                triples,
            }
        })
        .collect();

    // sort by subject
    subject_groups.sort_by(|a, b| a.subject.cmp(&b.subject));

    let mut context = Context::new();
    context.insert("title", "Definitions");
    context.insert(
        "index_href",
        index_path,
    );
    context.insert("subject_groups", &subject_groups);

    let html = tera.render("page", &context)?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(output_path, html)?;

    Ok(output_path.to_path_buf())
}

pub fn generate_index(
    output_dir: &str,
    name: &str,
    entries: &[IndexEntry],
    tera: &Tera,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut context = Context::new();
    context.insert("title", "Index of RDF Files");
    context.insert("entries", entries);

    let html = tera.render("index", &context)?;

    let index_path = Path::new(output_dir).join(name);
    fs::write(index_path, html)?;

    Ok(())
}

fn is_valid_url(s: &str) -> bool {
    Url::parse(s).is_ok()
}

/// Fragment of the IRI if present, otherwise the last non-empty path segment.
fn local_name_from_iri(iri: &str) -> String {
    let Ok(u) = Url::parse(iri) else {
        return String::new();
    };
    if let Some(f) = u.fragment() {
        return f.to_string();
    }
    u.path()
        .rsplit('/')
        .find(|s| !s.is_empty())
        .unwrap_or("")
        .to_string()
}

/// Safe HTML `id` / `#fragment`: alphanumerics, `_`, `-`, `.`, `:`; other chars → `-`.
fn sanitize_fragment_id(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_dash = false;
    for ch in raw.chars() {
        let mapped = match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '.' | ':' => Some(ch),
            '-' => Some('-'),
            ' ' | '/' | '\\' | '#' | '?' | '&' | '%' | '+' => Some('-'),
            _ if !ch.is_control() && ch.is_alphanumeric() => Some(ch),
            _ if !ch.is_control() => Some('-'),
            _ => None,
        };
        if let Some(c) = mapped {
            if c == '-' {
                if !out.is_empty() && !prev_dash {
                    out.push('-');
                    prev_dash = true;
                }
            } else {
                out.push(c);
                prev_dash = false;
            }
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Anchor from the RDF node name only (no hashes). Falls back to `subject_label`, then `"node"`.
fn anchor_for_subject(full_subject: &str, subject_label: &str) -> String {
    let raw = if full_subject.starts_with("_:") {
        full_subject
            .strip_prefix("_:")
            .unwrap_or(full_subject)
            .to_string()
    } else if is_valid_url(full_subject) {
        let n = local_name_from_iri(full_subject);
        if n.is_empty() {
            subject_label.to_string()
        } else {
            n
        }
    } else {
        subject_label.to_string()
    };

    let mut s = sanitize_fragment_id(&raw);
    if s.is_empty() {
        s = sanitize_fragment_id(subject_label);
    }
    if s.is_empty() {
        "node".into()
    } else {
        s
    }
}
