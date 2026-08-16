# The Artisan of Glimmith Solution Compendium

Extracts campaign puzzles from *The Artisan of Glimmith* and generates a PDF containing every solution.

## Requirements

- Rust toolchain
- [Typst](https://typst.app/)
- [just](https://github.com/casey/just)

## Generate the PDF

```sh
just pdf

# Set a different path when needed:
AOG_GAME_DIR="/path/to/The Artisan of Glimmith" just pdf
```

Output:

- `target/puzzles.json`: normalized puzzle data
- `target/aog-solutions.pdf`: solution compendium

## Other commands

```sh
Available recipes:
    build                                    # Build an optimized extractor binary.
    check                                    # Run the complete local verification suite.
    default
    extract game_dir=game_dir data=data_file # Extract normalized puzzle data from a game installation.
    fmt                                      # Format Rust sources.
    fmt-check                                # Verify Rust formatting without changing files.
    lint                                     # Treat every Clippy warning as an error.
    pdf game_dir=game_dir output=pdf_file    # Extract every puzzle and compile the vector solution compendium.
    svg data=data_file                       # Render each document page as an SVG for visual inspection.
    test                                     # Run all Rust tests.
```
