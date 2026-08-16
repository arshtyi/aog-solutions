use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: &str = "aog-solutions/1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    pub format: String,
    pub puzzles: Vec<Puzzle>,
}

impl Collection {
    pub fn new(puzzles: Vec<Puzzle>) -> Self {
        Self {
            format: FORMAT_VERSION.to_owned(),
            puzzles,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Puzzle {
    pub id: String,
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub difficulty: u32,
    pub cells: Vec<Cell>,
    pub empty_cells: Vec<Position>,
    pub solution_edges: Vec<Edge>,
    pub given_edges: Vec<Edge>,
    pub edge_clues: Vec<EdgeClue>,
    pub vertex_clues: Vec<VertexClue>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Position {
    pub row: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub row: u32,
    pub column: u32,
    pub kind: CellKind,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compass: Option<Compass>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellKind {
    Number,
    BorderedNumber,
    Compass,
    Shape,
    Region,
    Symbol,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compass {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Edge {
    pub orientation: Orientation,
    pub row: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeClue {
    pub orientation: Orientation,
    pub row: u32,
    pub column: u32,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VertexClue {
    pub x: u32,
    pub y: u32,
    pub symbol: String,
}
