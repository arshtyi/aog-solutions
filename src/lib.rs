pub mod discover;
pub mod extract;
pub mod model;
pub mod parser;
mod progress;

use std::path::{Path, PathBuf};

pub const DEFAULT_GAME_DIR: &str = "/mnt/c/Software/Steam/steamapps/common/The Artisan of Glimmith";
pub const DEFAULT_DATA_FILE: &str = "target/puzzles.json";

#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub game_dir: PathBuf,
    pub output: PathBuf,
    pub progress: bool,
}

impl ExtractOptions {
    pub fn new(game_dir: impl AsRef<Path>, output: impl AsRef<Path>) -> Self {
        Self {
            game_dir: game_dir.as_ref().to_owned(),
            output: output.as_ref().to_owned(),
            progress: true,
        }
    }

    pub fn with_progress(mut self, progress: bool) -> Self {
        self.progress = progress;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_the_supplied_game_path() {
        let supplied = Path::new(r"D:\SteamLibrary\The Artisan of Glimmith");
        let options = ExtractOptions::new(supplied, "target/puzzles.json");

        assert_eq!(options.game_dir, supplied);
    }

    #[test]
    fn defaults_to_the_wsl_mount_path() {
        assert_eq!(
            DEFAULT_GAME_DIR,
            "/mnt/c/Software/Steam/steamapps/common/The Artisan of Glimmith"
        );
    }
}
