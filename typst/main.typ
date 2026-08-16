#import "render.typ": palette, puzzle-sheet, strokes

#set page(
    paper: "a4",
    margin: (x: 16mm, y: 14mm),
    fill: palette.paper,
)
#set text(font: "Libertinus Serif", size: 10pt)
#set par(leading: .65em)

#let data-path = sys.inputs.at("data", default: none)
#let collection = if data-path == none {
    (format: "aog-solutions/2", puzzles: ())
} else {
    json(if data-path.starts-with("/") { data-path } else { "/" + data-path })
}

#assert(
    collection.format == "aog-solutions/2",
    message: "unsupported puzzle data format",
)

#let puzzles = collection.puzzles.sorted(key: puzzle => puzzle.game_id)

#align(center)[
    #v(36mm)
    #text(size: 25pt, weight: "bold", fill: palette.ink)[The Artisan of Glimmith]
    #v(4pt)
    #text(size: 13pt, fill: palette.border)[Puzzle Solution Compendium]
    #v(7pt)
    #box(width: 28mm, height: 2pt)[
        #place(center + horizon, line(length: 28mm, stroke: strokes.given))
    ]
    #v(18mm)
    #text(size: 9pt, fill: palette.border)[
        #collection.puzzles.len() puzzles
    ]
]

#let legend-line(given: false) = box(width: 14mm, height: 6mm)[
    #if given {
        place(
            center + horizon,
            line(length: 14mm, stroke: strokes.given-knockout),
        )
        place(
            center + horizon,
            line(length: 14mm, stroke: strokes.given),
        )
    } else {
        place(
            center + horizon,
            line(length: 14mm, stroke: strokes.solution),
        )
    }
]
#let swatch(fill, label: none, hatch: false, glyph: none) = box(
    width: 6mm,
    height: 6mm,
    fill: fill,
    stroke: (paint: palette.border, thickness: .55pt),
    clip: true,
)[
    #if hatch {
        place(
            center + horizon,
            rotate(-45deg, line(length: 5.4mm, stroke: strokes.hatch)),
        )
    }
    #if glyph != none {
        place(center + horizon, glyph)
    } else if label != none {
        place(
            center + horizon,
            text(size: 6pt, weight: "bold", fill: palette.ink, label),
        )
    }
]
#let legend-mark(body) = box(
    width: 14mm,
    height: 6mm,
    align(center + horizon, body),
)
#let legend-cell(
    fill,
    label: none,
    hatch: false,
    glyph: none,
) = legend-mark(
    swatch(
        fill,
        label: label,
        hatch: hatch,
        glyph: glyph,
    ),
)
#let compass-glyph = box(width: 4.7mm, height: 4.7mm)[
    #for angle in (45deg, -45deg) {
        place(
            center + horizon,
            rotate(
                angle,
                line(
                    length: 4.5mm,
                    stroke: (paint: palette.ink, thickness: .55pt, cap: "round"),
                ),
            ),
        )
    }
    #place(
        top + center,
        dy: .05mm,
        text(size: 2.8pt, weight: "bold", fill: palette.ink, "1"),
    )
    #place(
        right + horizon,
        dx: -.1mm,
        text(size: 2.8pt, weight: "bold", fill: palette.ink, "2"),
    )
    #place(
        bottom + center,
        dy: -.05mm,
        text(size: 2.8pt, weight: "bold", fill: palette.ink, "3"),
    )
    #place(
        left + horizon,
        dx: .1mm,
        text(size: 2.8pt, weight: "bold", fill: palette.ink, "4"),
    )
]

#let boundary-glyph = box(width: 4.2mm, height: 4.2mm, clip: true)[
    #let stroke = (
        paint: palette.light-ink,
        thickness: .9pt,
        cap: "round",
        join: "round",
    )
    #place(
        top + left,
        dx: .45mm,
        dy: .45mm,
        line(length: 3.3mm, stroke: stroke),
    )
    #place(
        top + left,
        dx: .45mm,
        dy: .45mm,
        line(length: 3.3mm, angle: 90deg, stroke: stroke),
    )
    #place(
        bottom + left,
        dx: .45mm,
        dy: -.45mm,
        line(length: 3.3mm, stroke: stroke),
    )
]

#let shape-tile = box(
    width: 1.25mm,
    height: 1.25mm,
    fill: palette.light-ink,
    stroke: (paint: palette.shape, thickness: .22pt),
)
#let polyomino-glyph = grid(
    columns: (1.25mm, 1.25mm, 1.25mm),
    rows: (1.25mm, 1.25mm),
    gutter: 0pt,
    [], shape-tile, [],
    shape-tile, shape-tile, shape-tile,
)
#let region-group-glyph = circle(
    radius: 1.9mm,
    fill: palette.panel,
    stroke: (paint: palette.ink, thickness: .65pt),
)[
    #align(
        center + horizon,
        text(size: 5pt, weight: "bold", fill: palette.ink, "1"),
    )
]

#let clue-dot(symbol) = circle(
    radius: 1.75mm,
    fill: palette.panel,
    stroke: (paint: palette.ink, thickness: .45pt),
)[
    #if symbol == "=" {
        place(
            center + horizon,
            box(width: 2.7mm, height: 1.8mm)[
                #let stroke = (paint: palette.ink, thickness: .55pt, cap: "round")
                #place(top + left, dy: .3mm, line(length: 2.7mm, stroke: stroke))
                #place(bottom + left, dy: -.3mm, line(length: 2.7mm, stroke: stroke))
            ],
        )
    } else {
        align(
            center + horizon,
            text(size: 4.5pt, weight: "bold", fill: palette.ink, symbol),
        )
    }
]
#let legend-clue(vertex: false) = legend-mark(box(width: 6mm, height: 6mm)[
    #let guide = (paint: palette.border, thickness: .5pt, cap: "round")
    #place(center + horizon, line(length: 5.4mm, stroke: guide))
    #if vertex {
        place(center + horizon, rotate(90deg, line(length: 5.4mm, stroke: guide)))
        place(center + horizon, clue-dot("2"))
    } else {
        place(center + horizon, clue-dot("="))
    }
])

#let legend-label(body) = text(size: 8.5pt, fill: palette.ink, body)
#let legend-heading(body) = text(
    size: 6.5pt,
    weight: "bold",
    tracking: .06em,
    fill: palette.border,
    upper(body),
)

#v(1fr)
#align(center)[
    #block(
        inset: (x: 8mm, y: 6mm),
        fill: palette.panel,
        stroke: (paint: palette.border, thickness: .55pt),
        radius: 1.6mm,
    )[
        #grid(
            columns: (14mm, 30mm, 10mm, 14mm, 34mm),
            column-gutter: 3mm,
            row-gutter: 2mm,
            align: (center + horizon, left + horizon),
            [], legend-heading("Board marks"), [], [], legend-heading("Clue cells"),
            legend-line(),
            legend-label[Solution border],
            [],
            legend-cell(palette.number, label: "12"),
            legend-label[Number],

            legend-line(given: true),
            legend-label[Given border],
            [],
            legend-cell(palette.compass-fill, glyph: compass-glyph),
            legend-label[Compass totals],

            legend-cell(palette.empty),
            legend-label[Playable cell],
            [],
            legend-cell(palette.shape, glyph: boundary-glyph),
            legend-label[Boundary shape],

            legend-cell(palette.outside, hatch: true),
            legend-label[Outside board],
            [],
            legend-cell(palette.shape, glyph: polyomino-glyph),
            legend-label[Polyomino shape],

            legend-clue(),
            legend-label[Edge clue],
            [],
            legend-cell(palette.region, glyph: region-group-glyph),
            legend-label[Region group],

            legend-clue(vertex: true), legend-label[Vertex clue], [], [], [],
        )
    ]
]

#if collection.puzzles.len() == 0 {
    v(12mm)
    align(center)[
        #text(fill: palette.border)[
            No data supplied. Compile with `--input data=target/puzzles.json`.
        ]
    ]
} else {
    pagebreak()
    set page(numbering: "1", number-align: center)

    for (index, puzzle) in puzzles.enumerate() {
        if index > 0 { pagebreak() }
        puzzle-sheet(puzzle)
    }
}
