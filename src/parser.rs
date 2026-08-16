use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};

use crate::model::{
    Cell, CellKind, Compass, Edge, EdgeClue, Orientation, Position, Puzzle, VertexClue,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Puzzle,
    Solution,
}

struct ParsedGrid {
    cells: Vec<Cell>,
    empty_cells: Vec<Position>,
    given_edges: Vec<Edge>,
    edge_clues: Vec<EdgeClue>,
    vertex_clues: Vec<VertexClue>,
}

pub fn parse(lines: &[String], source: &str, fallback_id: &str) -> Result<Puzzle> {
    let mut width = 0;
    let mut height = 0;
    let mut difficulty = 0;
    let mut public_id = None;
    let mut section = None;
    let mut puzzle_lines = Vec::new();
    let mut solution_lines = Vec::new();

    for raw in lines {
        let line = raw.trim_end_matches('\r');
        let trimmed = line.trim();

        if let Some((parsed_width, parsed_height)) = parse_dimensions(trimmed) {
            width = parsed_width;
            height = parsed_height;
        } else if let Some(value) = trimmed.strip_prefix("DIFFICULTY ") {
            difficulty = value
                .trim()
                .parse()
                .with_context(|| format!("invalid difficulty in {source}"))?;
        } else if let Some(value) = trimmed.strip_prefix("PUBLIC_ID ") {
            public_id = Some(value.trim().to_owned());
        }

        match trimmed {
            "PUZZLE" => section = Some(Section::Puzzle),
            "SOLUTION" => section = Some(Section::Solution),
            _ if !trimmed.is_empty() || (!line.is_empty() && line.bytes().all(|b| b == b' ')) => {
                match section {
                    Some(Section::Puzzle) => puzzle_lines.push(line.to_owned()),
                    Some(Section::Solution) => solution_lines.push(line.to_owned()),
                    None => {}
                }
            }
            _ => {}
        }
    }

    if width == 0 || height == 0 {
        bail!("missing or invalid dimensions in {source}");
    }
    if puzzle_lines.is_empty() || solution_lines.is_empty() {
        bail!("missing puzzle or solution section in {source}");
    }

    let ParsedGrid {
        cells,
        empty_cells,
        given_edges,
        edge_clues,
        vertex_clues,
    } = parse_puzzle_grid(&puzzle_lines, width, height);
    let solution_edges = parse_solution_edges(&solution_lines, width, height);

    if solution_edges.is_empty() {
        bail!("solution contains no edges in {source}");
    }

    let answer_set: BTreeSet<_> = solution_edges.iter().cloned().collect();
    if let Some(edge) = given_edges.iter().find(|edge| !answer_set.contains(*edge)) {
        bail!("given edge is absent from the solution in {source}: {edge:?}");
    }
    if let Some(clue) = edge_clues.iter().find(|clue| {
        !answer_set.contains(&Edge {
            orientation: clue.orientation,
            row: clue.row,
            column: clue.column,
        })
    }) {
        bail!("edge clue is absent from the solution in {source}: {clue:?}");
    }

    Ok(Puzzle {
        id: public_id
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| fallback_id.to_owned()),
        source: source.to_owned(),
        width,
        height,
        difficulty,
        cells,
        empty_cells,
        solution_edges,
        given_edges,
        edge_clues,
        vertex_clues,
    })
}

pub fn parse_dimensions(line: &str) -> Option<(u32, u32)> {
    let mut parts = line.split_whitespace();
    if parts.next()? != "DIMENSIONS" {
        return None;
    }
    let width = parts.next()?.parse().ok()?;
    let height = parts.next()?.parse().ok()?;
    (parts.next().is_none()).then_some((width, height))
}

fn parse_puzzle_grid(lines: &[String], width: u32, height: u32) -> ParsedGrid {
    let mut cells = BTreeMap::new();
    let mut empties = BTreeSet::new();
    let mut given_edges = BTreeSet::new();
    let mut edge_clues = Vec::new();
    let mut vertex_clues = Vec::new();
    let mut row = 0_u32;
    let mut horizontal_row = 0_u32;
    let mut seen_border = false;

    for line in lines {
        let bytes = line.as_bytes();
        if bytes.contains(&b'+') {
            seen_border = true;
            if horizontal_row > height {
                continue;
            }

            let mut vertices = Vec::new();
            for x in (0..bytes.len()).step_by(3) {
                let symbol = bytes[x];
                if symbol != b' ' {
                    vertices.push(x);
                    if symbol != b'+' && symbol != b'-' {
                        vertex_clues.push(VertexClue {
                            x: (x / 3) as u32,
                            y: horizontal_row,
                            symbol: char::from(symbol).to_string(),
                        });
                    }
                }
            }

            for pair in vertices.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let column = (a / 3) as u32;
                if column > width || a + 1 > b || b > bytes.len() {
                    continue;
                }
                let segment = &bytes[a + 1..b];
                if segment.len() >= 2 && segment[0] == b'#' {
                    given_edges.insert(Edge {
                        orientation: Orientation::Horizontal,
                        row: horizontal_row,
                        column,
                    });
                } else if let Some(&symbol) = segment.first()
                    && !matches!(symbol, b'-' | b' ' | b'|')
                {
                    edge_clues.push(EdgeClue {
                        orientation: Orientation::Horizontal,
                        row: horizontal_row,
                        column,
                        symbol: char::from(symbol).to_string(),
                    });
                }

                for (index, symbol) in segment.iter().copied().enumerate() {
                    let physical_offset = a + index + 1;
                    if symbol.is_ascii_digit() && !physical_offset.is_multiple_of(3) {
                        edge_clues.push(EdgeClue {
                            orientation: Orientation::Horizontal,
                            row: horizontal_row,
                            column,
                            symbol: char::from(symbol).to_string(),
                        });
                        break;
                    }
                }
            }
            horizontal_row += 1;
            continue;
        }

        let bars: Vec<_> = bytes
            .iter()
            .enumerate()
            .filter_map(|(index, &byte)| (byte == b'|').then_some(index))
            .collect();
        if bars.len() >= 2 {
            let leading = bytes
                .iter()
                .position(|byte| *byte != b' ')
                .unwrap_or(bytes.len());
            let mut logical_column = (leading / 3) as u32;

            for pair in bars.windows(2) {
                let content = &bytes[pair[0] + 1..pair[1]];
                parse_cell_segment(
                    content,
                    row,
                    &mut logical_column,
                    width,
                    height,
                    &mut cells,
                    &mut empties,
                    &mut given_edges,
                    &mut edge_clues,
                );
            }
            row += 1;
        } else if seen_border && !bytes.is_empty() && bytes.iter().all(|byte| *byte == b' ') {
            row += 1;
        }
    }

    for coordinate in cells.keys() {
        empties.remove(&Position {
            row: coordinate.0,
            column: coordinate.1,
        });
    }

    let mut cells: Vec<_> = cells.into_values().collect();
    cells.sort_by_key(|cell| (cell.row, cell.column));
    edge_clues.sort_by_key(|clue| (clue.orientation, clue.row, clue.column, clue.symbol.clone()));
    edge_clues.dedup_by(|left, right| left == right);
    vertex_clues.sort_by_key(|clue| (clue.y, clue.x, clue.symbol.clone()));
    vertex_clues.dedup_by(|left, right| left == right);

    ParsedGrid {
        cells,
        empty_cells: empties.into_iter().collect(),
        given_edges: given_edges.into_iter().collect(),
        edge_clues,
        vertex_clues,
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_cell_segment(
    content: &[u8],
    row: u32,
    logical_column: &mut u32,
    width: u32,
    height: u32,
    cells: &mut BTreeMap<(u32, u32), Cell>,
    empties: &mut BTreeSet<Position>,
    given_edges: &mut BTreeSet<Edge>,
    edge_clues: &mut Vec<EdgeClue>,
) {
    let mut cursor = 0;
    while cursor < content.len() {
        let byte = content[cursor];
        if content.get(cursor..cursor + 2) == Some(b"..") {
            let symbol = content.get(cursor + 2).copied();
            if symbol.is_some_and(is_edge_clue) {
                insert_empty(empties, row, *logical_column, width, height);
                *logical_column += 1;
                if *logical_column <= width {
                    edge_clues.push(EdgeClue {
                        orientation: Orientation::Vertical,
                        row,
                        column: *logical_column,
                        symbol: char::from(symbol.unwrap()).to_string(),
                    });
                }
                cursor += 3;
            } else {
                insert_empty(empties, row, *logical_column, width, height);
                *logical_column += 1;
                cursor += 2;
            }
        } else if byte == b'#' {
            let bordered_number = content
                .get(cursor + 1..cursor + 3)
                .filter(|value| value.iter().all(u8::is_ascii_digit));
            if let Some(value) = bordered_number {
                insert_cell(
                    cells,
                    row,
                    *logical_column,
                    width,
                    height,
                    CellKind::BorderedNumber,
                    value,
                );
                *logical_column += 1;
                cursor += 3;
            } else {
                if *logical_column <= width {
                    given_edges.insert(Edge {
                        orientation: Orientation::Vertical,
                        row,
                        column: *logical_column,
                    });
                }
                cursor += 1;
            }
        } else if matches!(byte, b'U' | b'D' | b'L' | b'R') {
            let stop = take_while(content, cursor, |value| {
                matches!(value, b'U' | b'D' | b'L' | b'R') || value.is_ascii_digit()
            });
            insert_cell(
                cells,
                row,
                *logical_column,
                width,
                height,
                CellKind::Compass,
                &content[cursor..stop],
            );
            *logical_column += 1;
            cursor = stop;
        } else if byte.is_ascii_digit() {
            let stop = take_while(content, cursor, u8::is_ascii_digit);
            insert_cell(
                cells,
                row,
                *logical_column,
                width,
                height,
                CellKind::Number,
                &content[cursor..stop],
            );
            *logical_column += 1;
            cursor = stop;
        } else if matches!(byte, b'F' | b'S' | b'P') {
            let stop = if byte == b'F' {
                (cursor + 2).min(content.len())
            } else {
                take_while(content, cursor + 1, u8::is_ascii_digit)
            };
            let kind = match byte {
                b'P' => CellKind::Region,
                b'F' | b'S' => CellKind::Shape,
                _ => unreachable!(),
            };
            insert_cell(
                cells,
                row,
                *logical_column,
                width,
                height,
                kind,
                &content[cursor..stop],
            );
            *logical_column += 1;
            cursor = stop;

            if byte == b'F' {
                while content.get(cursor).is_some_and(u8::is_ascii_digit) {
                    if *logical_column <= width {
                        edge_clues.push(EdgeClue {
                            orientation: Orientation::Vertical,
                            row,
                            column: *logical_column,
                            symbol: char::from(content[cursor]).to_string(),
                        });
                    }
                    cursor += 1;
                }
            }
        } else if matches!(byte, b'=' | b'<' | b'>' | b'!' | b'^' | b'v') {
            if *logical_column <= width {
                edge_clues.push(EdgeClue {
                    orientation: Orientation::Vertical,
                    row,
                    column: *logical_column,
                    symbol: char::from(byte).to_string(),
                });
            }
            cursor += 1;
        } else if byte == b' ' {
            let stop = take_while(content, cursor, |value| *value == b' ');
            *logical_column += (stop - cursor).div_ceil(3) as u32;
            cursor = stop;
        } else {
            insert_cell(
                cells,
                row,
                *logical_column,
                width,
                height,
                CellKind::Symbol,
                &content[cursor..cursor + 1],
            );
            *logical_column += 1;
            cursor += 1;
        }
    }
}

fn parse_solution_edges(lines: &[String], width: u32, height: u32) -> Vec<Edge> {
    let mut edges = BTreeSet::new();
    let mut horizontal_row = 0_u32;
    let mut vertical_row = 0_u32;
    let mut previous_vertices = None;

    for line in lines {
        let bytes = line.as_bytes();
        let vertices: Vec<_> = bytes
            .iter()
            .enumerate()
            .filter_map(|(index, &byte)| (byte == b'+').then_some(index))
            .collect();

        if vertices.len() > 1 {
            if horizontal_row <= height {
                for pair in vertices.windows(2) {
                    let (a, b) = (pair[0], pair[1]);
                    let column = (a / 3) as u32;
                    let segment = &bytes[a + 1..b];
                    if column < width && segment.starts_with(b"##") {
                        edges.insert(Edge {
                            orientation: Orientation::Horizontal,
                            row: horizontal_row,
                            column,
                        });
                    }
                }
                horizontal_row += 1;
            }
            previous_vertices = Some(vertices);
        } else if vertical_row < height
            && let Some(vertices) = previous_vertices.as_ref()
        {
            for &x in vertices {
                let column = (x / 3) as u32;
                if column <= width && bytes.get(x) == Some(&b'#') {
                    edges.insert(Edge {
                        orientation: Orientation::Vertical,
                        row: vertical_row,
                        column,
                    });
                }
            }
            vertical_row += 1;
        }
    }

    edges.into_iter().collect()
}

fn insert_empty(empties: &mut BTreeSet<Position>, row: u32, column: u32, width: u32, height: u32) {
    if row < height && column < width {
        empties.insert(Position { row, column });
    }
}

fn insert_cell(
    cells: &mut BTreeMap<(u32, u32), Cell>,
    row: u32,
    column: u32,
    width: u32,
    height: u32,
    kind: CellKind,
    text: &[u8],
) {
    if row >= height || column >= width {
        return;
    }
    let text = String::from_utf8_lossy(text).into_owned();
    let compass = (kind == CellKind::Compass).then(|| parse_compass(&text));
    cells.insert(
        (row, column),
        Cell {
            row,
            column,
            kind,
            text,
            compass,
        },
    );
}

fn parse_compass(text: &str) -> Compass {
    let bytes = text.as_bytes();
    let mut compass = Compass::default();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let direction = bytes[cursor];
        cursor += 1;
        let start = cursor;
        cursor = take_while(bytes, cursor, u8::is_ascii_digit);
        if cursor == start {
            continue;
        }
        let value = text[start..cursor].parse().ok();
        match direction {
            b'U' => compass.up = value,
            b'D' => compass.down = value,
            b'L' => compass.left = value,
            b'R' => compass.right = value,
            _ => {}
        }
    }
    compass
}

fn take_while(content: &[u8], start: usize, predicate: impl Fn(&u8) -> bool) -> usize {
    let mut cursor = start;
    while content.get(cursor).is_some_and(&predicate) {
        cursor += 1;
    }
    cursor
}

fn is_edge_clue(symbol: u8) -> bool {
    matches!(symbol, b'=' | b'<' | b'>' | b'!') || symbol.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<String> {
        [
            "DIMENSIONS 2 2",
            "DIFFICULTY 3",
            "PUBLIC_ID 12345",
            "PUZZLE",
            "+##+--+",
            "|02=F3|",
            "+--+--+",
            "|..#P2|",
            "+--+--+",
            "SOLUTION",
            "+##+  +",
            "#  #  #",
            "+  +##+",
            "#  #  #",
            "+##+##+",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }

    #[test]
    fn parses_reference_format() {
        let puzzle = parse(&sample(), "fixture", "fallback").unwrap();

        assert_eq!(puzzle.id, "12345");
        assert_eq!((puzzle.width, puzzle.height, puzzle.difficulty), (2, 2, 3));
        assert_eq!(puzzle.cells.len(), 3);
        assert_eq!(puzzle.empty_cells, vec![Position { row: 1, column: 0 }]);
        assert!(puzzle.given_edges.contains(&Edge {
            orientation: Orientation::Vertical,
            row: 1,
            column: 1,
        }));
        assert!(puzzle.edge_clues.contains(&EdgeClue {
            orientation: Orientation::Vertical,
            row: 0,
            column: 1,
            symbol: "=".to_owned(),
        }));
    }

    #[test]
    fn parses_compass_values() {
        assert_eq!(
            parse_compass("U0D12LR3"),
            Compass {
                up: Some(0),
                down: Some(12),
                left: None,
                right: Some(3),
            }
        );
    }
}
