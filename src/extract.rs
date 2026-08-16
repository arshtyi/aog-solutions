use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

use crate::ExtractOptions;
use crate::discover::discover;
use crate::model::{Collection, Puzzle};
use crate::parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtractReport {
    pub puzzles: usize,
    pub scanned_sources: usize,
    pub archives: usize,
}

pub fn run(options: &ExtractOptions) -> Result<ExtractReport> {
    if !options.game_dir.is_dir() {
        bail!(
            "game directory does not exist: {}",
            options.game_dir.display()
        );
    }

    let mut files = collect_files(&options.game_dir)?;
    files.sort();
    let archives: Vec<_> = files
        .iter()
        .filter(|path| extension(path) == Some("pak"))
        .cloned()
        .collect();
    let loose_files: Vec<_> = files
        .into_iter()
        .filter(|path| extension(path) != Some("pak") && is_candidate_path(path))
        .collect();

    let mut puzzles = Vec::new();
    let mut scanned_sources = 0;

    for archive in &archives {
        scan_archive(
            &options.game_dir,
            archive,
            &mut scanned_sources,
            &mut puzzles,
        )?;
    }
    for path in loose_files {
        let data = fs::read(&path)
            .with_context(|| format!("failed to read loose asset {}", path.display()))?;
        let label = relative_label(&options.game_dir, &path);
        scanned_sources += 1;
        parse_source(&label, &data, &mut puzzles);
    }

    deduplicate(&mut puzzles);
    puzzles.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.source.cmp(&right.source))
    });
    make_ids_unique(&mut puzzles);

    if puzzles.is_empty() {
        bail!(
            "no serialized puzzle solutions were found in {} (scanned {} candidate sources across {} PAK archives)",
            options.game_dir.display(),
            scanned_sources,
            archives.len()
        );
    }

    let puzzle_count = puzzles.len();
    write_collection(&options.output, &Collection::new(puzzles))?;
    Ok(ExtractReport {
        puzzles: puzzle_count,
        scanned_sources,
        archives: archives.len(),
    })
}

fn scan_archive(
    game_dir: &Path,
    archive_path: &Path,
    scanned_sources: &mut usize,
    puzzles: &mut Vec<Puzzle>,
) -> Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("failed to open PAK archive {}", archive_path.display()))?;
    let mut reader = BufReader::new(file);
    let archive = repak::PakBuilder::new()
        .reader(&mut reader)
        .with_context(|| format!("failed to read PAK index {}", archive_path.display()))?;
    let mut entries = archive.files();
    entries.sort();
    let archive_label = relative_label(game_dir, archive_path);

    for entry in entries
        .into_iter()
        .filter(|entry| is_candidate_entry(entry))
    {
        let data = archive.get(&entry, &mut reader).with_context(|| {
            format!(
                "failed to decompress {entry} from {}",
                archive_path.display()
            )
        })?;
        let label = format!("{archive_label}::{entry}");
        *scanned_sources += 1;
        parse_source(&label, &data, puzzles);
    }
    Ok(())
}

fn parse_source(label: &str, data: &[u8], puzzles: &mut Vec<Puzzle>) {
    let raw_puzzles = discover(data);
    let multiple = raw_puzzles.len() > 1;
    for (index, raw) in raw_puzzles.into_iter().enumerate() {
        let fallback = fallback_id(label, index, multiple);
        let source = format!("{label}@0x{:x}", raw.offset);
        if let Ok(puzzle) = parser::parse(&raw.lines, &source, &fallback) {
            puzzles.push(puzzle);
        }
    }
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(entry) if entry.file_type().is_file() => Some(Ok(entry.into_path())),
            Ok(_) => None,
            Err(error) => Some(Err(error.into())),
        })
        .collect()
}

fn is_candidate_entry(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("puzzle")
        || ["uasset", "uexp", "umap", "txt", "json", "csv", "sav", "bin"]
            .iter()
            .any(|extension| lower.ends_with(&format!(".{extension}")))
}

fn is_candidate_path(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    lower.contains("puzzle")
        || matches!(
            extension(path),
            Some("uasset" | "uexp" | "umap" | "txt" | "json" | "csv" | "sav" | "bin")
        )
}

fn extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|value| value.to_str())
}

fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn fallback_id(source: &str, index: usize, multiple: bool) -> String {
    let entry = source.rsplit("::").next().unwrap_or(source);
    let file_name = entry.rsplit(['/', '\\']).next().unwrap_or(entry);
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |value| value.0);
    if multiple {
        format!("{stem}-{:03}", index + 1)
    } else {
        stem.to_owned()
    }
}

fn deduplicate(puzzles: &mut Vec<Puzzle>) {
    let mut signatures = BTreeSet::new();
    puzzles.retain(|puzzle| {
        let mut value = puzzle.clone();
        value.id.clear();
        value.source.clear();
        signatures.insert(serde_json::to_string(&value).expect("puzzle model is serializable"))
    });
}

fn make_ids_unique(puzzles: &mut [Puzzle]) {
    let mut used = BTreeSet::new();
    for puzzle in puzzles {
        let base = puzzle.id.clone();
        let mut suffix = 2;
        while !used.insert(puzzle.id.clone()) {
            puzzle.id = format!("{base}-{suffix}");
            suffix += 1;
        }
    }
}

fn write_collection(path: &Path, collection: &Collection) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let file = File::create(path)
        .with_context(|| format!("failed to create output file {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, collection)
        .context("failed to serialize puzzle collection")?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const SAMPLE: &str = "DIMENSIONS 1 1\nDIFFICULTY 2\nPUBLIC_ID 54321\nPUZZLE\n+--+\n|02|\n+--+\nSOLUTION\n+##+\n#  #\n+##+\n";

    #[test]
    fn extracts_a_loose_puzzle_and_writes_normalized_json() {
        let temporary = tempdir().unwrap();
        let input = temporary.path().join("Geri/Content/Puzzles");
        fs::create_dir_all(&input).unwrap();
        fs::write(input.join("sample.txt"), SAMPLE).unwrap();
        let output = temporary.path().join("out/puzzles.json");

        let report = run(&ExtractOptions::new(temporary.path(), &output)).unwrap();
        let collection: Collection =
            serde_json::from_reader(BufReader::new(File::open(output).unwrap())).unwrap();

        assert_eq!(report.puzzles, 1);
        assert_eq!(report.archives, 0);
        assert_eq!(collection.puzzles[0].id, "54321");
        assert_eq!(collection.puzzles[0].solution_edges.len(), 4);
    }

    #[test]
    fn extracts_a_puzzle_from_an_unreal_pak() {
        let temporary = tempdir().unwrap();
        let pak_dir = temporary.path().join("Geri/Content/Paks");
        fs::create_dir_all(&pak_dir).unwrap();
        let pak_path = pak_dir.join("pakchunk0-WindowsNoEditor.pak");
        let writer = File::create(&pak_path).unwrap();
        let mut pak = repak::PakBuilder::new().writer(
            writer,
            repak::Version::V11,
            "../../../".to_owned(),
            Some(0),
        );
        pak.write_file("Geri/Content/Puzzles/sample.txt", false, SAMPLE)
            .unwrap();
        pak.write_index().unwrap();
        let output = temporary.path().join("puzzles.json");

        let report = run(&ExtractOptions::new(temporary.path(), output)).unwrap();

        assert_eq!(report.puzzles, 1);
        assert_eq!(report.archives, 1);
        assert_eq!(report.scanned_sources, 1);
    }
}
