use crate::parser::parse_dimensions;

const MAX_STRING_UNITS: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPuzzle {
    pub offset: usize,
    pub lines: Vec<String>,
}

#[derive(Debug)]
struct Fragment {
    start: usize,
    end: usize,
    rank: u8,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Header,
    Puzzle,
    Solution,
}

#[derive(Debug)]
struct Candidate {
    offset: usize,
    height: u32,
    phase: Phase,
    solution_boundaries: u32,
    lines: Vec<String>,
}

pub fn discover(data: &[u8]) -> Vec<RawPuzzle> {
    let mut fragments = unreal_strings(data);
    fragments.extend(ascii_runs(data));
    fragments.extend(utf16_ascii_runs(data));
    fragments.sort_by_key(|fragment| (fragment.start, std::cmp::Reverse(fragment.rank)));

    let mut selected: Vec<Fragment> = Vec::new();
    for fragment in fragments {
        let duplicate = selected.iter().any(|existing| {
            existing.start <= fragment.start
                && existing.end >= fragment.end
                && existing.text == fragment.text
        });
        if !duplicate {
            selected.push(fragment);
        }
    }
    selected.sort_by_key(|fragment| fragment.start);

    let mut result = Vec::new();
    let mut candidate: Option<Candidate> = None;
    for fragment in selected {
        for line in split_lines(&fragment.text) {
            if let Some((_, height)) = parse_dimensions(line.trim()) {
                candidate = Some(Candidate {
                    offset: fragment.start,
                    height,
                    phase: Phase::Header,
                    solution_boundaries: 0,
                    lines: vec![line],
                });
                continue;
            }

            let Some(active) = candidate.as_mut() else {
                continue;
            };
            let trimmed = line.trim();
            match active.phase {
                Phase::Header => {
                    if trimmed == "PUZZLE" {
                        active.lines.push(line);
                        active.phase = Phase::Puzzle;
                    } else if is_header_line(trimmed) {
                        active.lines.push(line);
                    }
                }
                Phase::Puzzle => {
                    if trimmed == "SOLUTION" {
                        active.lines.push(line);
                        active.phase = Phase::Solution;
                    } else if is_grid_line(&line) {
                        active.lines.push(line);
                    }
                }
                Phase::Solution => {
                    if !is_grid_line(&line) {
                        continue;
                    }
                    if line.bytes().filter(|byte| *byte == b'+').count() > 1 {
                        active.solution_boundaries += 1;
                    }
                    active.lines.push(line);
                    if active.solution_boundaries == active.height + 1 {
                        let finished = candidate.take().unwrap();
                        result.push(RawPuzzle {
                            offset: finished.offset,
                            lines: finished.lines,
                        });
                    }
                }
            }
        }
    }
    result
}

fn unreal_strings(data: &[u8]) -> Vec<Fragment> {
    let mut result = Vec::new();
    if data.len() < 5 {
        return result;
    }

    for start in 0..=data.len() - 4 {
        let units = i32::from_le_bytes(data[start..start + 4].try_into().unwrap());
        if units > 0 {
            let length = units as usize;
            if length > MAX_STRING_UNITS {
                continue;
            }
            let Some(end) = start.checked_add(4 + length) else {
                continue;
            };
            if end > data.len() || data[end - 1] != 0 {
                continue;
            }
            let bytes = &data[start + 4..end - 1];
            if let Ok(text) = std::str::from_utf8(bytes)
                && valid_text(text)
            {
                result.push(Fragment {
                    start,
                    end,
                    rank: 3,
                    text: text.to_owned(),
                });
            }
        } else if units < 0 {
            let Some(length) = units.checked_abs().map(|value| value as usize) else {
                continue;
            };
            if length > MAX_STRING_UNITS {
                continue;
            }
            let Some(byte_length) = length.checked_mul(2) else {
                continue;
            };
            let Some(end) = start.checked_add(4 + byte_length) else {
                continue;
            };
            if end > data.len() || data[end - 2..end] != [0, 0] {
                continue;
            }
            let units: Vec<_> = data[start + 4..end - 2]
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            if let Ok(text) = String::from_utf16(&units)
                && valid_text(&text)
            {
                result.push(Fragment {
                    start,
                    end,
                    rank: 3,
                    text,
                });
            }
        }
    }
    result
}

fn ascii_runs(data: &[u8]) -> Vec<Fragment> {
    let mut result = Vec::new();
    let mut cursor = 0;
    while cursor < data.len() {
        if !is_text_byte(data[cursor]) {
            cursor += 1;
            continue;
        }
        let start = cursor;
        while cursor < data.len() && is_text_byte(data[cursor]) {
            cursor += 1;
        }
        if cursor - start >= 2
            && let Ok(text) = std::str::from_utf8(&data[start..cursor])
            && valid_text(text)
        {
            result.push(Fragment {
                start,
                end: cursor,
                rank: 1,
                text: text.to_owned(),
            });
        }
    }
    result
}

fn utf16_ascii_runs(data: &[u8]) -> Vec<Fragment> {
    let mut result = Vec::new();
    let mut cursor = 0;
    while cursor + 1 < data.len() {
        if !is_text_byte(data[cursor]) || data[cursor + 1] != 0 {
            cursor += 1;
            continue;
        }
        let start = cursor;
        let mut bytes = Vec::new();
        while cursor + 1 < data.len() && is_text_byte(data[cursor]) && data[cursor + 1] == 0 {
            bytes.push(data[cursor]);
            cursor += 2;
        }
        if bytes.len() >= 2 {
            let text = String::from_utf8(bytes).unwrap();
            if valid_text(&text) {
                result.push(Fragment {
                    start,
                    end: cursor,
                    rank: 2,
                    text,
                });
            }
        }
    }
    result
}

fn split_lines(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split('\n')
        .map(|line| line.trim_end_matches('\r').to_owned())
}

fn is_header_line(line: &str) -> bool {
    ["VERSION ", "DIFFICULTY ", "PUBLIC_ID ", "TITLE ", "AUTHOR "]
        .iter()
        .any(|prefix| line.starts_with(prefix))
}

fn is_grid_line(line: &str) -> bool {
    let bars = line.bytes().filter(|byte| *byte == b'|').count();
    line.contains('+')
        || line.contains('#')
        || bars >= 2
        || (!line.is_empty() && line.bytes().all(|byte| byte == b' '))
}

fn valid_text(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|character| matches!(character, '\t' | '\n' | '\r') || !character.is_control())
}

fn is_text_byte(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | b'\r' | b' '..=b'~')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_fstring(output: &mut Vec<u8>, value: &str) {
        let length = i32::try_from(value.len() + 1).unwrap();
        output.extend_from_slice(&length.to_le_bytes());
        output.extend_from_slice(value.as_bytes());
        output.push(0);
    }

    #[test]
    fn recovers_a_puzzle_from_unreal_strings() {
        let lines = [
            "DIMENSIONS 1 1",
            "DIFFICULTY 2",
            "PUZZLE",
            "+--+",
            "|02|",
            "+--+",
            "SOLUTION",
            "+##+",
            "#  #",
            "+##+",
        ];
        let mut data = vec![0xff; 17];
        for line in lines {
            push_fstring(&mut data, line);
            data.extend_from_slice(&[0xde, 0xad]);
        }

        let puzzles = discover(&data);

        assert_eq!(puzzles.len(), 1);
        assert_eq!(puzzles[0].lines, lines);
    }

    #[test]
    fn recovers_a_plain_multiline_block() {
        let data = b"noise\0DIMENSIONS 1 1\nDIFFICULTY 1\nPUZZLE\n+--+\n|..|\n+--+\nSOLUTION\n+##+\n#  #\n+##+\0tail";

        assert_eq!(discover(data).len(), 1);
    }
}
