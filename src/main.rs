mod parser;

use crate::parser::{convert_file, generate_index, IndexEntry};

use clap::{Arg, Command};
use std::fs;
use tera::Tera;
use walkdir::WalkDir;

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
                    index_entries.push(IndexEntry::new(
                        rel_path.to_string_lossy().to_string(),
                        path.file_name().unwrap().to_string_lossy().to_string(),
                    ));
                }
                Err(e) => eprintln!("Error converting file {:?}: {}", path, e),
            }
        }
    }

    generate_index(output_dir, &index_entries, &tera)?;

    Ok(())
}
