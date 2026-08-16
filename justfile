set dotenv-load := false

default:
    @just --list

fmt:
    cargo fmt

fmt-check:
    cargo fmt --all -- --check

check: fmt-check
    cargo check --all-targets

typst-check:
    mkdir -p target/typst-check
    typst compile --root . typst/main.typ target/typst-check/document.pdf
