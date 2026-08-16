set dotenv-load := false

game_dir := env("AOG_GAME_DIR", "/mnt/c/Software/Steam/steamapps/common/The Artisan of Glimmith")
data_file := "target/puzzles.json"
pdf_file := "target/aog-solutions.pdf"

default:
    @just --list

# Format Rust sources.
fmt:
    cargo fmt

# Verify Rust formatting without changing files.
fmt-check:
    cargo fmt --all -- --check

# Run all Rust tests.
test:
    cargo test --all-targets

# Treat every Clippy warning as an error.
lint:
    cargo clippy --all-targets -- -D warnings

# Run the complete local verification suite.
check: fmt-check test lint

# Build an optimized extractor binary.
build:
    cargo build --release

# Extract normalized puzzle data from a game installation.
extract game_dir=game_dir data=data_file:
    cargo run --release -- extract --game-dir "{{ game_dir }}" --output "{{ data }}"

# Extract every puzzle and compile the vector solution compendium.
pdf game_dir=game_dir output=pdf_file:
    @echo "[pipeline 1/2] Extracting puzzle data..."
    just extract "{{ game_dir }}" "{{ data_file }}"
    @echo "[pipeline 2/2] Typesetting vector PDF..."
    typst compile --root . --input "data={{ data_file }}" typst/main.typ "{{ output }}"
    @echo "Done: {{ output }}"

# Render each document page as an SVG for visual inspection.
svg data=data_file:
    mkdir -p target/svg
    @echo "Rendering vector SVG pages..."
    typst compile --root . --input "data={{ data }}" typst/main.typ "target/svg/solution-{0p}.svg"
    @echo "Done: target/svg/"
