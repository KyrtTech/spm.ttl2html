use clap::{Arg, Command};
use rio_api::model::{NamedNode, Subject, Term};
use rio_api::parser::TriplesParser;
use rio_turtle::{TurtleError, TurtleParser};
use serde::Serialize;
use std::fs;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tera::{Context, Tera};
use url::Url;
use walkdir::WalkDir;

#[derive(Serialize, Debug)]
struct Triple {
    subject: String,
    predicate: String,
    object: String,

    subject_link: Option<String>,
    subject_label: String,
    predicate_link: Option<String>,
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
        }
    }
}

#[derive(Serialize)]
struct SubjectGroup {
    subject: String,
    subject_label: String,
    subject_link: Option<String>,
    triples: Vec<Triple>,
}

#[derive(Serialize)]
struct IndexEntry {
    path: String,
    name: String,
}

fn is_valid_url(s: &str) -> bool {
    Url::parse(s).is_ok()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("RDF to HTML Converter")
        .version("0.1.0")
        .author("Radu Dita <radu@kyrt.tech>")
        .about("Converts RDF Turtle files to HTML")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("INPUT_DIR")
                .help("Sets the input directory")
                .required(true)
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("OUTPUT_DIR")
                .help("Sets the output directory")
                .required(true)
        )
        .get_matches();

    let input_dir = matches.get_one::<String>("input").unwrap();
    let output_dir = matches.get_one::<String>("output").unwrap();

    fs::create_dir_all(output_dir)?;

    let mut tera = Tera::default();
    tera.add_raw_template("page", include_str!("../templates/page.html"))
        .expect("Failed to add template");
    tera.add_raw_template("index", include_str!("../templates/index.html"))
        .expect("Failed to add index template");

    let mut index_entries = Vec::new();

    for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("ttl") {
            println!("Converting file: {:?}", path);
            match convert_file(path, input_dir, output_dir, &tera) {
                Ok(rel_path) => {
                    println!("Successfully converted {:?}", path);
                    index_entries.push(IndexEntry {
                        path: rel_path.to_string_lossy().to_string(),
                        name: path.file_name().unwrap().to_string_lossy().to_string(),
                    });
                }
                Err(e) => eprintln!("Error converting file {:?}: {}", path, e),
            }
        }
    }

    generate_index(output_dir, &index_entries, &tera)?;

    Ok(())
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
}

fn convert_file(
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

fn generate_index(
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
