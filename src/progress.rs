use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

const BAR_WIDTH: usize = 24;
const UPDATE_INTERVAL: Duration = Duration::from_millis(80);

pub(crate) struct Progress {
    enabled: bool,
    interactive: bool,
    line_open: bool,
    last_update: Instant,
}

impl Progress {
    pub(crate) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            interactive: io::stderr().is_terminal(),
            line_open: false,
            last_update: Instant::now() - UPDATE_INTERVAL,
        }
    }

    pub(crate) fn stage(&mut self, current: usize, total: usize, message: &str) {
        if !self.enabled {
            return;
        }
        self.clear_line();
        eprintln!("[{current}/{total}] {message}");
    }

    pub(crate) fn note(&mut self, message: impl std::fmt::Display) {
        if !self.enabled {
            return;
        }
        self.clear_line();
        eprintln!("      {message}");
    }

    pub(crate) fn archive(&mut self, current: usize, total: usize, label: &str, entries: usize) {
        self.note(format_args!(
            "PAK {current}/{total}: {label} ({entries} candidate entries)"
        ));
    }

    pub(crate) fn scan(&mut self, current: usize, total: usize, puzzles: usize, label: &str) {
        if !self.enabled || !self.interactive {
            return;
        }
        let now = Instant::now();
        if current != total && now.duration_since(self.last_update) < UPDATE_INTERVAL {
            return;
        }
        self.last_update = now;
        let line = scan_line(current, total, puzzles, label);
        eprint!("\r\x1b[2K{line}");
        let _ = io::stderr().flush();
        self.line_open = true;
    }

    pub(crate) fn scan_done(&mut self, scanned: usize, puzzles: usize) {
        if !self.enabled {
            return;
        }
        self.clear_line();
        eprintln!("      Scanned {scanned} sources; found {puzzles} puzzle blocks");
    }

    pub(crate) fn complete(&mut self, puzzles: usize, output: impl std::fmt::Display) {
        if !self.enabled {
            return;
        }
        self.clear_line();
        eprintln!("      Wrote {puzzles} puzzles to {output}");
    }

    fn clear_line(&mut self) {
        if self.line_open {
            eprint!("\r\x1b[2K");
            let _ = io::stderr().flush();
            self.line_open = false;
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        self.clear_line();
    }
}

fn scan_line(current: usize, total: usize, puzzles: usize, label: &str) -> String {
    let completed = current
        .saturating_mul(BAR_WIDTH)
        .checked_div(total)
        .unwrap_or(BAR_WIDTH)
        .min(BAR_WIDTH);
    let percent = current
        .saturating_mul(100)
        .checked_div(total)
        .unwrap_or(100)
        .min(100);
    let bar = format!(
        "{}{}",
        "#".repeat(completed),
        "-".repeat(BAR_WIDTH - completed)
    );
    format!(
        "      [{bar}] {percent:>3}% {current}/{total} · {puzzles} puzzles · {}",
        shorten(label, 64)
    )
}

fn shorten(value: &str, limit: usize) -> String {
    let length = value.chars().count();
    if length <= limit {
        return value.to_owned();
    }
    let tail: String = value.chars().skip(length - limit + 1).collect();
    format!("…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_scan_progress() {
        let line = scan_line(25, 100, 7, "asset.puz");

        assert!(line.contains("25% 25/100"));
        assert!(line.contains("7 puzzles"));
        assert!(line.contains("[######------------------]"));
    }

    #[test]
    fn keeps_the_end_of_long_asset_paths() {
        let shortened = shorten(
            "Geri/Content/Puzzles/A/VeryLongPuzzleNameThatMatters.puz",
            24,
        );

        assert_eq!(shortened.chars().count(), 24);
        assert!(shortened.starts_with('…'));
        assert!(shortened.ends_with("ThatMatters.puz"));
    }
}
