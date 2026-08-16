#import "@preview/cetz:0.5.2"

// The clue colors are display-space conversions of representative glass colors
// from the game's GridPaletteData asset. Deep blue ink and warm paper echo the
// official puzzle UI while glyphs, hatching, and dash patterns keep every
// distinction legible without color.
#let palette = (
    paper: rgb("#f7f2e8"),
    panel: rgb("#fffdf8"),
    outside: rgb("#ded8ce"),
    outside-hatch: rgb("#7b8990"),
    empty: rgb("#fffdf8"),
    number: rgb("#66d9e4"),
    compass-fill: rgb("#f59b61"),
    shape: rgb("#8c6b8f"),
    region: rgb("#ffdf34"),
    ink: rgb("#17313d"),
    light-ink: rgb("#ffffff"),
    solution: rgb("#17313d"),
    given: rgb("#c7422f"),
    border: rgb("#52636b"),
)

#let given-dash = (array: (3.6pt, 2.1pt), phase: 0pt)
#let strokes = (
    solution: (
        paint: palette.solution,
        thickness: 2.35pt,
        cap: "round",
        join: "round",
    ),
    given-knockout: (
        paint: palette.panel,
        thickness: 2.8pt,
        dash: given-dash,
        cap: "round",
    ),
    given: (
        paint: palette.given,
        thickness: 1.65pt,
        dash: given-dash,
        cap: "round",
    ),
    hatch: (paint: palette.outside-hatch, thickness: .55pt, cap: "round"),
)

#let cell-fill(kind) = if kind == "number" {
    palette.number
} else if kind == "compass" {
    palette.compass-fill
} else if kind == "shape" {
    palette.shape
} else if kind == "region" {
    palette.region
} else {
    palette.empty
}

#let cell-ink(kind) = if kind == "shape" {
    palette.light-ink
} else {
    palette.ink
}

// These are source-format tokens, not display glyphs. The game uses ^/v for
// vertical relations and ! for not-equal in .puz files; drawing those literal
// characters would misrepresent the official relation marks.
#let relation-tokens = ("=", "!", "<", ">", "^", "v")
#let numeric-clue-tokens = ("0", "1", "2", "3", "4", "5", "6", "7", "8", "9")

#let draw-puzzle(puzzle, cell-size: 9mm) = cetz.canvas(length: cell-size, {
    import cetz.draw: *

    let height = puzzle.height
    let label-size = calc.max(5pt, cell-size * 0.23)
    let clue-size = calc.max(4.5pt, cell-size * 0.19)
    let point(row, column) = (column, height - row)
    let active-cells = (
        puzzle.empty_cells.map(cell => (cell.row, cell.column)) + puzzle.cells.map(cell => (cell.row, cell.column))
    )
    let draw-shape-glyph(cell, x, y) = {
        let ink = cell-ink(cell.kind)
        let glyph = cell.at("glyph", default: none)
        assert(
            glyph != none,
            message: "missing derived shape glyph: " + cell.text,
        )
        if glyph.kind == "boundary" {
            let inset = .25
            let stroke = (
                paint: ink,
                thickness: 1.25pt,
                cap: "round",
                join: "round",
            )
            if glyph.top {
                line(
                    (x + inset, y + 1 - inset),
                    (x + 1 - inset, y + 1 - inset),
                    stroke: stroke,
                )
            }
            if glyph.right {
                line(
                    (x + 1 - inset, y + inset),
                    (x + 1 - inset, y + 1 - inset),
                    stroke: stroke,
                )
            }
            if glyph.bottom {
                line((x + inset, y + inset), (x + 1 - inset, y + inset), stroke: stroke)
            }
            if glyph.left {
                line((x + inset, y + inset), (x + inset, y + 1 - inset), stroke: stroke)
            }
        } else if glyph.kind == "polyomino" {
            let unit = .64 / calc.max(glyph.width, glyph.height)
            let glyph-width = glyph.width * unit
            let glyph-height = glyph.height * unit
            let origin-x = x + .5 - glyph-width / 2
            let origin-y = y + .5 - glyph-height / 2
            for part in glyph.cells {
                let part-x = origin-x + part.column * unit
                let part-y = origin-y + (glyph.height - part.row - 1) * unit
                rect(
                    (part-x, part-y),
                    (part-x + unit, part-y + unit),
                    fill: ink,
                    stroke: (paint: palette.shape, thickness: .24pt),
                )
            }
        } else {
            assert(false, message: "unsupported shape glyph: " + glyph.kind)
        }
    }

    // A light hatch distinguishes off-grid space without a large, ink-heavy
    // gray block. Active cells cover the background and its hatch marks.
    rect(
        (0, 0),
        (puzzle.width, puzzle.height),
        fill: palette.outside,
        stroke: none,
    )
    for row in range(puzzle.height) {
        for column in range(puzzle.width) {
            if not ((row, column) in active-cells) {
                let y = height - row - 1
                line(
                    (column + .18, y + .12),
                    (column + .88, y + .82),
                    stroke: strokes.hatch,
                )
            }
        }
    }

    for cell in puzzle.empty_cells {
        let x = cell.column
        let y = height - cell.row - 1
        rect((x, y), (x + 1, y + 1), fill: palette.empty, stroke: none)
    }

    for cell in puzzle.cells {
        let x = cell.column
        let y = height - cell.row - 1
        rect((x, y), (x + 1, y + 1), fill: cell-fill(cell.kind), stroke: none)

        if cell.kind == "compass" {
            let compass-stroke = (
                paint: cell-ink(cell.kind),
                thickness: .75pt,
                cap: "round",
            )
            line((x, y), (x + 1, y + 1), stroke: compass-stroke)
            line((x + 1, y), (x, y + 1), stroke: compass-stroke)
            let values = cell.compass
            let compass-label(key, position) = {
                let value = values.at(key, default: none)
                if value != none {
                    content(
                        position,
                        text(
                            size: clue-size,
                            weight: "bold",
                            fill: cell-ink(cell.kind),
                            str(value),
                        ),
                    )
                }
            }
            compass-label("up", (x + .5, y + .76))
            compass-label("down", (x + .5, y + .24))
            compass-label("left", (x + .24, y + .5))
            compass-label("right", (x + .76, y + .5))
        } else if cell.kind == "shape" {
            draw-shape-glyph(cell, x, y)
        } else if cell.kind == "region" {
            circle(
                (x + .5, y + .5),
                radius: .25,
                fill: none,
                stroke: (paint: cell-ink(cell.kind), thickness: .7pt),
            )
            content(
                (x + .5, y + .5),
                text(
                    size: label-size,
                    weight: "bold",
                    fill: cell-ink(cell.kind),
                    cell.text.slice(1),
                ),
            )
        } else {
            assert(
                cell.kind == "number",
                message: "unsupported cell kind: " + cell.kind,
            )
            content(
                (x + .5, y + .5),
                text(
                    size: label-size,
                    weight: "bold",
                    fill: cell-ink(cell.kind),
                    cell.text,
                ),
            )
        }
    }

    let draw-edge(edge, stroke) = {
        if edge.orientation == "horizontal" {
            line(
                point(edge.row, edge.column),
                point(edge.row, edge.column + 1),
                stroke: stroke,
            )
        } else {
            line(
                point(edge.row, edge.column),
                point(edge.row + 1, edge.column),
                stroke: stroke,
            )
        }
    }

    for edge in puzzle.solution_edges {
        if not (edge in puzzle.given_edges) {
            draw-edge(edge, strokes.solution)
        }
    }

    // Given edges are their own rounded vermilion dash, matching the official
    // dashed-border language without the visually noisy solid-line underlay.
    for edge in puzzle.given_edges {
        draw-edge(edge, strokes.given-knockout)
        draw-edge(edge, strokes.given)
    }

    let draw-relation(position, token) = {
        let (x, y) = position
        let stroke = (
            paint: palette.ink,
            thickness: .8pt,
            cap: "round",
            join: "round",
        )
        if token == "=" or token == "!" {
            line((x - .075, y + .045), (x + .075, y + .045), stroke: stroke)
            line((x - .075, y - .045), (x + .075, y - .045), stroke: stroke)
            if token == "!" {
                line((x - .055, y - .095), (x + .055, y + .095), stroke: stroke)
            }
        } else {
            let (tip, first, second) = if token == "<" {
                ((x - .065, y), (x + .055, y + .08), (x + .055, y - .08))
            } else if token == ">" {
                ((x + .065, y), (x - .055, y + .08), (x - .055, y - .08))
            } else if token == "^" {
                ((x, y + .08), (x - .065, y - .065), (x + .065, y - .065))
            } else {
                ((x, y - .08), (x - .065, y + .065), (x + .065, y + .065))
            }
            line(first, tip, stroke: stroke)
            line(tip, second, stroke: stroke)
        }
    }

    let draw-clue(position, symbol) = {
        circle(
            position,
            radius: .19,
            fill: palette.panel,
            stroke: (paint: palette.ink, thickness: .55pt),
        )
        if symbol in relation-tokens {
            draw-relation(position, symbol)
        } else {
            assert(
                symbol in numeric-clue-tokens,
                message: "unsupported clue token: " + symbol,
            )
            content(
                position,
                text(size: clue-size, weight: "bold", fill: palette.ink, symbol),
            )
        }
    }

    for clue in puzzle.edge_clues {
        let position = if clue.orientation == "horizontal" {
            (clue.column + .5, height - clue.row)
        } else {
            (clue.column, height - clue.row - .5)
        }
        draw-clue(position, clue.symbol)
    }

    for clue in puzzle.vertex_clues {
        draw-clue((clue.x, height - clue.y), clue.symbol)
    }
})

#let puzzle-sheet(puzzle) = {
    let drawing-width = 172mm
    let drawing-height = 218mm
    let cell-size = calc.min(
        16mm,
        drawing-width / puzzle.width,
        drawing-height / puzzle.height,
    )

    align(center)[
        #text(size: 18pt, weight: "bold", fill: palette.ink)[Puzzle #puzzle.game_id]
        #v(2pt)
        #text(size: 8.5pt, fill: palette.border)[
            Grid #puzzle.width × #puzzle.height · Difficulty #puzzle.difficulty
        ]
    ]
    v(6mm)
    align(center, draw-puzzle(puzzle, cell-size: cell-size))
}
