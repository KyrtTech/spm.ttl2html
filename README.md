# RDF Converter

`RDF Converter` is a command-line tool that converts RDF Turtle files into HTML files. The tool processes RDF files and generates human-readable HTML representations, making it easier to view and share RDF data.

## Table of Contents

- [Installation](#installation)
- [Usage](#usage)
- [Options](#options)
- [Examples](#examples)
- [License](#license)

## Installation

To install this tool, you need to have [Rust](https://www.rust-lang.org/) installed on your machine. If you don't have Rust installed, you can install it by following the instructions [here](https://www.rust-lang.org/tools/install).

Clone the repository and build the project:

```bash
git clone https://github.com/yourusername/rdf-converter.git
cd rdf-converter
cargo build --release
```

After building the project, you'll find the executable in the target/release directory.

## Usage
The basic usage of the RDF Converter is as follows:

```bash
turtle2rdf --input <INPUT_DIR> --output <OUTPUT_DIR>
```

## Options

* `-i, --input <INPUT_DIR>`: Specifies the input directory containing the RDF Turtle files.
* `-o, --output <OUTPUT_DIR>`: Specifies the output directory where the generated HTML files will be saved.
* `-h, --help`: Prints help information.
* `-V, --version`: Prints the version information.

## Examples
Convert RDF Turtle files located in the ontology directory to HTML files and save them in the output directory:

```bash
turtle2rdf --input ./ontology --output ./output
```

This command will process all `.ttl` files in the `ontology` directory, generate corresponding HTML files, and store them in the output directory. 

It also generates an `index.html` files that acts as a `TOC` with links to all the individual generated files.

## License

This project is licensed under the MIT License. See the **LICENSE** file for more details.
