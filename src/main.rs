mod manifest;
mod parser;

use crate::manifest::Manifest;
use crate::parser::{convert_file, generate_index, IndexEntry};

use clap::{Arg, Command};
use std::{fs, path::Path};
use tera::Tera;
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("RDF to HTML Converter")
        .version("0.1.1")
        .author("Radu Dita <radu@kyrt.tech>")
        .about("Converts RDF Turtle files to HTML")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("INPUT_DIR")
                .help("Sets the input directory")
                .required(true),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("OUTPUT_DIR")
                .help("Sets the output directory")
                .required(true),
        )
        .arg(
            Arg::new("manifest")
                .short('m')
                .long("manifest")
                .value_name("MANIFEST_FILE")
                .help("Sets the manifest file")
                .required(false),
        )
        .get_matches();

    let input_dir = matches.get_one::<String>("input").unwrap();
    let output_dir = matches.get_one::<String>("output").unwrap();
    let manifest: Manifest = if let Some(path) = matches.get_one::<String>("manifest") {
        let raw = fs::read_to_string(path)?;
        serde_json::from_str(&raw)?
    } else {
        Manifest::default()
    };

    let publish_config = manifest.publish;

    fs::create_dir_all(output_dir)?;

    let mut tera = Tera::default();
    tera.add_raw_template("page", include_str!("../templates/page.html"))
        .expect("Failed to add template");
    tera.add_raw_template("index", include_str!("../templates/index.html"))
        .expect("Failed to add index template");

    let mut index_entries = Vec::new();
    let index_name = publish_config
        .as_ref()
        .and_then(|config| config.index.clone())
        .unwrap_or_else(|| "index.html".to_string());

    for entry in WalkDir::new(input_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("ttl") {
            println!("Converting file: {:?}", path);
            let relative_path = path.strip_prefix(input_dir)?.with_extension("").to_string_lossy().to_string();
            let output_path = Path::new(output_dir)
                .join(
                    manifest
                        .routes
                        .get(&relative_path)
                        .unwrap_or(&relative_path)
                )
                .with_extension("html");
            match convert_file(path, &output_path, &tera, &publish_config) {
                Ok(rel_path) => {
                    println!("Successfully converted {:?}", path);
                    index_entries.push(IndexEntry::new(
                        rel_path.to_string_lossy().to_string().replace(output_dir, ""),
                        path.with_extension("").file_name().unwrap().to_string_lossy().to_string(),
                    ));
                }
                Err(e) => eprintln!("Error converting file {:?}: {}", path, e),
            }
        }
    }

    generate_index(output_dir, &index_name, &index_entries, &tera)?;

    Ok(())
}
