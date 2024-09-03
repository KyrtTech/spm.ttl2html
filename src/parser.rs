use std::collections::HashMap;
use std::fs;

use std::path::{Path, PathBuf};

use rio_api::model::{NamedNode, Subject, Term};
use rio_api::parser::TriplesParser;
use rio_turtle::{TurtleError, TurtleParser};

use tera::{Context, Tera};

use serde::Serialize;
use url::Url;

#[derive(Serialize, Debug)]
pub struct Triple {
    subject: String,
    predicate: String,
    object: String,

    subject_link: Option<String>,
    subject_label: String,
    predicate_link: Option<String>,
    object_link: Option<String>,
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
        }
    }
}

#[derive(Serialize)]
pub struct SubjectGroup {
    subject: String,
    subject_label: String,
    subject_link: Option<String>,
    triples: Vec<Triple>,
}

#[derive(Serialize)]
pub struct IndexEntry {
    path: String,
    name: String,
}

impl IndexEntry {
    pub fn new(name: String, path: String) -> Self {
        IndexEntry { name, path }
    }
}

fn update_triple_with_links(triple: &mut Triple, prefixes: &Vec<&String>) {
    if is_valid_url(&triple.subject) {
        for prefix in prefixes {
            if triple.subject.starts_with(*prefix) {
                triple.subject_link = Some(triple.subject.clone());
                triple.subject_label = triple.subject.replace(*prefix, "");

                break;
            }
        }
    }

    if is_valid_url(&triple.predicate) {
        for prefix in prefixes {
            if triple.predicate.starts_with(*prefix) {
                triple.predicate_link = Some(triple.predicate.clone());
                triple.predicate = triple.predicate.replace(*prefix, "");

                break;
            }
        }
    }

    if is_valid_url(&triple.object) {
        for prefix in prefixes {
            if triple.object.starts_with(*prefix) {
                triple.object_link = Some(triple.object.clone());
                triple.object = triple.object.replace(*prefix, "");

                break;
            }
        }
    }
}

pub fn convert_file(
    input_path: &Path,
    input_dir: &str,
    output_dir: &str,
    tera: &Tera,
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
                Term::Literal(literal) => literal.to_string(),
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
            };

            unparsed_triples.push(triple);

            Ok::<(), TurtleError>(())
        });

        let prefixes = parser.prefixes().values().collect::<Vec<&String>>();

        for mut triple in unparsed_triples {
            update_triple_with_links(&mut triple, &prefixes);

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
        .map(|(subject, triples)| SubjectGroup {
            subject,
            subject_link: triples[0].subject_link.clone(),
            subject_label: triples[0].subject_label.clone(),
            triples,
        })
        .collect();
    subject_groups.sort_by(|a, b| a.subject.cmp(&b.subject));

    let mut context = Context::new();
    context.insert("title", "Definitions");
    context.insert("subject_groups", &subject_groups);

    let html = tera.render("page", &context)?;

    let relative_path = input_path.strip_prefix(input_dir)?;
    let output_path = Path::new(output_dir)
        .join(relative_path)
        .with_extension("html");

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(output_path, html)?;

    Ok(relative_path.with_extension("html").to_path_buf())
}

pub fn generate_index(
    output_dir: &str,
    entries: &[IndexEntry],
    tera: &Tera,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut context = Context::new();
    context.insert("title", "Index of RDF Files");
    context.insert("entries", entries);

    let html = tera.render("index", &context)?;

    let index_path = Path::new(output_dir).join("index.html");
    fs::write(index_path, html)?;

    Ok(())
}

fn is_valid_url(s: &str) -> bool {
    Url::parse(s).is_ok()
}
