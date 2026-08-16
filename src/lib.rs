pub mod discover;
pub mod extract;
pub mod model;
pub mod parser;

use std::path::{Path, PathBuf};

pub const DEFAULT_GAME_DIR: &str = r"C:/Software/Steam/steamapps/common/The Artisan of Glimmith";
pub const DEFAULT_DATA_FILE: &str = "target/puzzles.json";

#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub game_dir: PathBuf,
    pub output: PathBuf,
}

impl ExtractOptions {
    pub fn new(game_dir: impl AsRef<Path>, output: impl AsRef<Path>) -> Self {
        Self {
            game_dir: game_dir.as_ref().to_owned(),
            output: output.as_ref().to_owned(),
        }
    }
}
