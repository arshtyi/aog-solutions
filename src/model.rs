use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub format: String,
    pub puzzles: Vec<Puzzle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Puzzle {
    pub id: String,
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub difficulty: u32,
}
