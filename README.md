# turtle2html

`turtle2html` is a small CLI that walks a directory of **RDF Turtle** (`.ttl`) files and generates **static HTML** for each file, plus an **`index.html`** table of contents. It is intended for browsing ontology and entity data in a browser (local files or any static host).

## Table of contents

- [Installation](#installation)
- [Usage](#usage)
- [CLI options](#cli-options)
- [Manifest (`-m`)](#manifest--m)
- [Generated HTML behavior](#generated-html-behavior)
- [Examples](#examples)
- [License](#license)

## Installation

Requires [Rust](https://www.rust-lang.org/). From this crate’s directory:

```bash
cd spm.turtle2rdf
cargo build --release
```

The binary is `target/release/turtle2html`. You can also run without installing:

```bash
cargo run -- --input <INPUT_DIR> --output <OUTPUT_DIR>
```

## Usage

```bash
turtle2html --input <INPUT_DIR> --output <OUTPUT_DIR> [--manifest <MANIFEST_FILE>]
```

Paths under `INPUT_DIR` are scanned recursively; every file with extension `.ttl` is converted to a matching `.html` path under `OUTPUT_DIR`.

## CLI options

| Option | Description |
|--------|-------------|
| `-i`, `--input <INPUT_DIR>` | Root directory containing `.ttl` files (required). |
| `-o`, `--output <OUTPUT_DIR>` | Directory where HTML output is written (required). |
| `-m`, `--manifest <MANIFEST_FILE>` | Optional JSON manifest for **output routes** and **published link rewriting** (see below). |
| `-h`, `--help` | Print help. |
| `-V`, `--version` | Print version. |

## Manifest (`-m`)

The manifest is a JSON file with two optional top-level sections.

### `routes`

Maps each input file’s **relative path without `.ttl`** to the **relative output path without `.html`**.

- Keys and values use forward-slash style segments (e.g. `entities/application` → `entities/app`).
- Files not listed keep the same relative path as the input (only the extension changes to `.html`).

### `publish` (optional)

When present, ontology IRIs in link `href`s can be rewritten for deployment:

- **`url`**: Base URL prefix used in generated links (e.g. your static docs site). Trailing slash is optional; it is normalized when matching “internal” links for [new-tab behavior](#generated-html-behavior).
- **`ontology_prefix`**: Prefix of IRIs in the Turtle that should be replaced by `url` (e.g. `http://the-spm.org/`). Predicate and object links whose IRI starts with this prefix are rewritten before being written to HTML.
- **`iri_routes`**: Map from an ontology **document path** (the path segment in the IRI after `ontology_prefix`, before `#`) to the **published document path** (relative to `url`, without `.html`). Use this when Turtle namespace paths differ from the HTML paths produced by `routes` (for example `entities/apps` in RDF vs `entities/app` on disk). If a path is not listed, the document path is left unchanged after the prefix swap. An empty object `{}` is valid.
- **`should_use_extesion_for_links`**: When `true`, `.html` is appended to the document path in rewritten links after `iri_routes` is applied. When `false`, only the prefix swap and `iri_routes` remapping run.

Link rewriting order: replace `ontology_prefix` with `url`, remap the document path with `iri_routes`, then append `.html` when `should_use_extesion_for_links` is `true`. URI fragments (`#localName`) are preserved.

If `publish` is omitted, links keep the full IRIs from the Turtle (still useful for local preview).

### Example manifest

See **`manifest.example.json`** in this directory for a copy-paste template. Minimal illustration:

```json
{
  "routes": {
    "schema": "vocab/schema",
    "entities/application": "entities/app",
    "entities/cloud_provider": "entities/cloud-providers"
  },
  "publish": {
    "url": "https://docs.example.com/ontology/",
    "ontology_prefix": "http://the-spm.org/",
    "should_use_extesion_for_links": true,
    "iri_routes": {
      "entities/apps": "entities/app",
      "entities/cloud_providers": "entities/cloud-providers"
    }
  }
}
```

Run:

```bash
cargo run -- \
  --input ../spm.knowledge_domain/ontology \
  --output ./output \
  --manifest ./manifest.example.json
```

## Generated HTML behavior

- **Per-file pages**: A sticky **On this page** roster on the left lists every definition and links to its in-page anchor; the main column groups each subject’s triples in a fixed-width **predicate / object** table with wrapping IRIs.
- **Links**: Prefixes from each Turtle file (plus built-in RDF/RDFS vocabulary prefixes) shorten labels where possible; `http`/`https` links that are **not** under `publish.url` open in a **new tab** and show a small **external-link** icon.
- **Back to top**: After scrolling down (~320px), a **floating button** appears to scroll smoothly to the top (`prefers-reduced-motion` uses instant scroll).
- **Index**: `index.html` in the output directory lists all generated pages.

## Examples

Convert an ontology tree without a manifest (output paths mirror input paths):

```bash
cargo run -- --input ./ontology --output ./html-out
```

Convert with routing and published base URL (adjust paths to your machine):

```bash
cargo run -- \
  -i ../spm.knowledge_domain/ontology \
  -o ./output \
  -m ./manifest.json
```

## License

This project is licensed under the MIT License. See the **LICENSE** file if present in the repository.
