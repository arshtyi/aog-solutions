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
use crate::progress::Progress;

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

    let mut progress = Progress::new(options.progress);
    progress.stage(1, 3, "Indexing game files...");
    let mut files = collect_files(&options.game_dir)?;
    files.sort();
    let archives: Vec<_> = files
        .iter()
        .filter(|path| has_extension(path, "pak"))
        .cloned()
        .collect();
    let active_puzzles = collect_active_puzzle_names(&archives)?;
    if let Some(active) = &active_puzzles {
        progress.note(format_args!(
            "Found {} active puzzle references in the campaign map",
            active.len()
        ));
    }
    let loose_files: Vec<_> = files
        .into_iter()
        .filter(|path| {
            !has_extension(path, "pak")
                && is_candidate_path(path)
                && is_active_loose_path(path, active_puzzles.as_ref())
        })
        .collect();
    progress.note(format_args!(
        "Found {} PAK archives and {} loose candidate files",
        archives.len(),
        loose_files.len()
    ));

    let mut puzzles = Vec::new();
    let mut scanned_sources = 0;

    progress.stage(2, 3, "Extracting and parsing puzzle data...");
    for (index, archive) in archives.iter().enumerate() {
        scan_archive(
            &options.game_dir,
            archive,
            (index + 1, archives.len()),
            &mut scanned_sources,
            &mut puzzles,
            &mut progress,
            active_puzzles.as_ref(),
        )?;
    }
    if !loose_files.is_empty() {
        progress.note(format_args!(
            "Loose assets: {} candidate files",
            loose_files.len()
        ));
    }
    for (index, path) in loose_files.iter().enumerate() {
        let label = relative_label(&options.game_dir, path);
        progress.scan(index, loose_files.len(), puzzles.len(), &label);
        let data = fs::read(path)
            .with_context(|| format!("failed to read loose asset {}", path.display()))?;
        scanned_sources += 1;
        parse_source(&label, &data, &mut puzzles)?;
        progress.scan(index + 1, loose_files.len(), puzzles.len(), &label);
    }
    progress.scan_done(scanned_sources, puzzles.len());

    progress.stage(3, 3, "Normalizing and writing puzzle data...");
    deduplicate(&mut puzzles);
    puzzles.sort_by(|left, right| {
        left.game_id
            .cmp(&right.game_id)
            .then_with(|| left.source.cmp(&right.source))
    });
    ensure_unique_game_ids(&puzzles)?;

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
    progress.complete(puzzle_count, options.output.display());
    Ok(ExtractReport {
        puzzles: puzzle_count,
        scanned_sources,
        archives: archives.len(),
    })
}

fn scan_archive(
    game_dir: &Path,
    archive_path: &Path,
    archive_position: (usize, usize),
    scanned_sources: &mut usize,
    puzzles: &mut Vec<Puzzle>,
    progress: &mut Progress,
    active_puzzles: Option<&BTreeSet<String>>,
) -> Result<()> {
    let (archive_index, archive_count) = archive_position;
    let archive_label = relative_label(game_dir, archive_path);
    progress.note(format_args!(
        "Opening PAK {archive_index}/{archive_count}: {archive_label}"
    ));
    let file = File::open(archive_path)
        .with_context(|| format!("failed to open PAK archive {}", archive_path.display()))?;
    let mut reader = BufReader::new(file);
    let archive = repak::PakBuilder::new()
        .reader(&mut reader)
        .with_context(|| format!("failed to read PAK index {}", archive_path.display()))?;
    let mut entries: Vec<_> = archive
        .files()
        .into_iter()
        .filter(|entry| is_candidate_entry(entry, active_puzzles))
        .collect();
    entries.sort();
    progress.archive(archive_index, archive_count, &archive_label, entries.len());

    let entry_count = entries.len();
    let puzzles_before = puzzles.len();
    for (index, entry) in entries.into_iter().enumerate() {
        progress.scan(index, entry_count, puzzles.len(), &entry);
        let data = archive.get(&entry, &mut reader).with_context(|| {
            format!(
                "failed to decompress {entry} from {}",
                archive_path.display()
            )
        })?;
        let label = format!("{archive_label}::{entry}");
        *scanned_sources += 1;
        parse_source(&label, &data, puzzles)?;
        progress.scan(index + 1, entry_count, puzzles.len(), &entry);
    }
    progress.note(format_args!(
        "Completed {archive_label}: {} puzzles found",
        puzzles.len() - puzzles_before
    ));
    Ok(())
}

fn collect_active_puzzle_names(archives: &[PathBuf]) -> Result<Option<BTreeSet<String>>> {
    let mut result = BTreeSet::new();
    let mut available = BTreeSet::new();
    for archive_path in archives {
        let file = File::open(archive_path)
            .with_context(|| format!("failed to open PAK archive {}", archive_path.display()))?;
        let mut reader = BufReader::new(file);
        let archive = repak::PakBuilder::new()
            .reader(&mut reader)
            .with_context(|| format!("failed to read PAK index {}", archive_path.display()))?;
        available.extend(
            archive
                .files()
                .into_iter()
                .filter(|entry| is_candidate_entry(entry, None))
                .map(|entry| file_name(&entry).to_ascii_lowercase()),
        );
        for entry in archive
            .files()
            .into_iter()
            .filter(|entry| is_campaign_puzzle_index(entry))
        {
            let data = archive.get(&entry, &mut reader).with_context(|| {
                format!(
                    "failed to decompress campaign puzzle index {entry} from {}",
                    archive_path.display()
                )
            })?;
            result.extend(puzzle_reference_names(&data));
        }
    }
    result.retain(|name| available.contains(name));
    Ok((!result.is_empty()).then_some(result))
}

fn is_campaign_puzzle_index(path: &str) -> bool {
    file_name(path).eq_ignore_ascii_case("Overworld_Campaign.uexp")
}

fn puzzle_reference_names(data: &[u8]) -> BTreeSet<String> {
    let mut result = BTreeSet::new();

    let mut cursor = 0;
    while cursor < data.len() {
        if !data[cursor].is_ascii_graphic() && data[cursor] != b' ' {
            cursor += 1;
            continue;
        }
        let start = cursor;
        while cursor < data.len() && (data[cursor].is_ascii_graphic() || data[cursor] == b' ') {
            cursor += 1;
        }
        collect_reference_names(&data[start..cursor], &mut result);
    }

    let mut cursor = 0;
    while cursor + 1 < data.len() {
        if (!data[cursor].is_ascii_graphic() && data[cursor] != b' ')
            || data[cursor + 1] != 0
            || (cursor > 0 && data[cursor - 1].is_ascii_graphic())
        {
            cursor += 1;
            continue;
        }
        let mut text = Vec::new();
        while cursor + 1 < data.len()
            && (data[cursor].is_ascii_graphic() || data[cursor] == b' ')
            && data[cursor + 1] == 0
        {
            text.push(data[cursor]);
            cursor += 2;
        }
        collect_reference_names(&text, &mut result);
    }

    result
}

fn collect_reference_names(text: &[u8], result: &mut BTreeSet<String>) {
    let lower = text.to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(offset) = lower[cursor..]
        .windows(4)
        .position(|window| window == b".puz")
    {
        let extension = cursor + offset;
        let end = extension + 4;
        let start = lower[..extension]
            .iter()
            .rposition(|byte| !is_puzzle_name_byte(*byte))
            .map_or(0, |index| index + 1);
        if start < extension {
            result.insert(String::from_utf8_lossy(&lower[start..end]).into_owned());
        }
        cursor = end;
    }
}

fn is_puzzle_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
}

fn parse_source(label: &str, data: &[u8], puzzles: &mut Vec<Puzzle>) -> Result<()> {
    let raw_puzzles = discover(data);
    let multiple = raw_puzzles.len() > 1;
    for (index, raw) in raw_puzzles.into_iter().enumerate() {
        let fallback = fallback_id(label, index, multiple);
        let source = format!("{label}@0x{:x}", raw.offset);
        let puzzle = parser::parse(&raw.lines, &source, &fallback)
            .with_context(|| format!("failed to parse puzzle at {source}"))?;
        puzzles.push(puzzle);
    }
    Ok(())
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

fn is_candidate_entry(path: &str, active_puzzles: Option<&BTreeSet<String>>) -> bool {
    let is_puzzle = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("puz"));
    is_puzzle
        && active_puzzles
            .is_none_or(|active| active.contains(&file_name(path).to_ascii_lowercase()))
}

fn is_active_loose_path(path: &Path, active_puzzles: Option<&BTreeSet<String>>) -> bool {
    active_puzzles.is_none_or(|active| {
        has_extension(path, "puz")
            && path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| active.contains(&name.to_ascii_lowercase()))
    })
}

fn is_candidate_path(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    lower.contains("puzzle")
        || matches!(
            extension(path).map(str::to_ascii_lowercase).as_deref(),
            Some("puz" | "uasset" | "uexp" | "umap" | "txt" | "json" | "csv" | "sav" | "bin")
        )
}

fn extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|value| value.to_str())
}

fn has_extension(path: &Path, expected: &str) -> bool {
    extension(path).is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
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
        value.game_id.clear();
        value.resource_id.clear();
        value.source.clear();
        signatures.insert(serde_json::to_string(&value).expect("puzzle model is serializable"))
    });
}

fn ensure_unique_game_ids(puzzles: &[Puzzle]) -> Result<()> {
    let mut used = BTreeSet::new();
    for puzzle in puzzles {
        if !used.insert(&puzzle.game_id) {
            bail!(
                "duplicate game puzzle ID {} (resource {})",
                puzzle.game_id,
                puzzle.resource_id,
            );
        }
    }
    Ok(())
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
        assert_eq!(collection.puzzles[0].game_id, "54321");
        assert_eq!(collection.puzzles[0].resource_id, "sample");
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
        pak.write_file("Geri/Content/Puzzles/sample.puz", false, SAMPLE)
            .unwrap();
        pak.write_index().unwrap();
        let output = temporary.path().join("puzzles.json");

        let report = run(&ExtractOptions::new(temporary.path(), output)).unwrap();

        assert_eq!(report.puzzles, 1);
        assert_eq!(report.archives, 1);
        assert_eq!(report.scanned_sources, 1);
    }

    #[test]
    fn limits_pak_scanning_to_puzzle_assets() {
        assert!(is_candidate_entry(
            "Geri/Content/AGeri/Puzzles/Zone1/0001.puz",
            None,
        ));
        assert!(!is_candidate_entry(
            "Geri/Content/AGeri/Puzzles/Metadata.uasset",
            None,
        ));
        assert!(!is_candidate_entry(
            "Engine/Content/Animation/DefaultAnimBoneCompressionSettings.uasset",
            None,
        ));
        assert!(!is_candidate_entry(
            "Geri/Content/AGeri/Textures/PuzzleBackdrop.png",
            None,
        ));
    }

    #[test]
    fn discovers_ascii_and_utf16_campaign_references() {
        let mut data = b"noise\0AGeri/Puzzles/Zone1/0008.puz\0".to_vec();
        data.extend(
            "0052.PUZ"
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .chain([0, 0]),
        );

        let references = puzzle_reference_names(&data);

        assert_eq!(
            references,
            BTreeSet::from(["0008.puz".into(), "0052.puz".into()])
        );
    }

    #[test]
    fn filters_pak_entries_to_the_campaign_reference_list() {
        let temporary = tempdir().unwrap();
        let pak_dir = temporary.path().join("Geri/Content/Paks");
        fs::create_dir_all(&pak_dir).unwrap();
        let writer = File::create(pak_dir.join("pakchunk0-WindowsNoEditor.pak")).unwrap();
        let mut pak = repak::PakBuilder::new().writer(
            writer,
            repak::Version::V11,
            "../../../".to_owned(),
            Some(0),
        );
        pak.write_file(
            "AGeri/Maps/PuzzleLevelMainSubmaps/Overworld_Campaign.uexp",
            false,
            b"Geri/Content/AGeri/Puzzles/Zone1/active.puz\0missing.puz\0",
        )
        .unwrap();
        pak.write_file("Geri/Content/AGeri/Puzzles/Zone1/active.puz", false, SAMPLE)
            .unwrap();
        pak.write_file("Geri/Content/AGeri/Puzzles/Zone1/stale.puz", false, SAMPLE)
            .unwrap();
        pak.write_index().unwrap();

        let output = temporary.path().join("puzzles.json");
        let report = run(&ExtractOptions::new(temporary.path(), &output)).unwrap();
        let collection: Collection =
            serde_json::from_reader(BufReader::new(File::open(output).unwrap())).unwrap();

        assert_eq!(report.puzzles, 1);
        assert_eq!(report.scanned_sources, 1);
        assert!(collection.puzzles[0].source.contains("active.puz"));
    }

    #[test]
    #[ignore = "downloads the platform Oodle runtime on first use"]
    fn extracts_a_puzzle_from_an_oodle_pak() {
        let temporary = tempdir().unwrap();
        let pak_dir = temporary.path().join("Geri/Content/Paks");
        fs::create_dir_all(&pak_dir).unwrap();
        let pak_path = pak_dir.join("pakchunk0-WindowsNoEditor.pak");
        let writer = File::create(&pak_path).unwrap();
        let mut pak = repak::PakBuilder::new()
            .compression([repak::Compression::Oodle])
            .writer(writer, repak::Version::V11, "../../../".to_owned(), Some(0));
        pak.write_file("Geri/Content/Puzzles/sample.puz", true, SAMPLE)
            .unwrap();
        pak.write_index().unwrap();

        let report = run(&ExtractOptions::new(
            temporary.path(),
            temporary.path().join("puzzles.json"),
        ))
        .unwrap();

        assert_eq!(report.puzzles, 1);
        assert_eq!(report.archives, 1);
        assert_eq!(report.scanned_sources, 1);
    }
}
