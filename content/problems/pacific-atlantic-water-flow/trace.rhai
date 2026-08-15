fn parse_rows(spec) {
    let out = [];
    for row in words(spec) {
        let cells = [];
        for v in row.split(",") { cells.push(num(v)); }
        out.push(cells);
    }
    out
}

fn render_rows(g) {
    let out = [];
    for row in g { out.push(join(row, ",")); }
    join(out, " ")
}

// Pacific Atlantic Water Flow — run the water backwards.
//
// Forwards, every cell needs its own search and the searches overlap
// enormously. Backwards, the question becomes "which cells drain into this
// ocean?", and one search per ocean answers it for the whole grid: start on
// the shore and climb, since water arriving here must have come from somewhere
// at least as high. The answer is then the intersection of two marked sets.

fn marked(seen) {
    let out = [];
    for r in 0..seen.len() {
        for c in 0..seen[0].len() { if seen[r][c] { out.push([r, c]); } }
    }
    out
}

fn flow(h, seen, r, c, label) {
    enter(`flow(${r},${c}) — ${label}`, #{ height: N(h[r][c]) });

    seen[r][c] = true;
    step("mark", `(${r},${c}), height ${h[r][c]}, drains to the ${label}.`,
        [ grid("heights", h).cur([r, c]).done(marked(seen)) ]);

    let dirs = [[1, 0], [-1, 0], [0, 1], [0, -1]];
    for d in dirs {
        let nr = r + d[0];
        let nc = c + d[1];
        step("neighbours", `Checking (${nr},${nc}).`,
            [ grid("heights", h).cur([r, c]).done(marked(seen)) ]);

        if nr >= 0 && nr < h.len() && nc >= 0 && nc < h[0].len() && !seen[nr][nc]
            && h[nr][nc] >= h[r][c] {
            step("uphill", `${h[nr][nc]} ≥ ${h[r][c]}, so water could flow from there to here — which means it also reaches the ${label}.`,
                [ grid("heights", h).cur([r, c]).hl([[nr, nc]]).done(marked(seen)) ]);
            step("recurse", `Climb to (${nr},${nc}).`,
                [ grid("heights", h).cur([nr, nc]).done(marked(seen)) ], "call");
            seen = flow(h, seen, nr, nc, label);
        } else {
            step("uphill",
                if nr < 0 || nr >= h.len() || nc < 0 || nc >= h[0].len() {
                    "Off the grid.".to_string()
                } else if seen[nr][nc] {
                    "Already known to drain there."
                } else {
                    `${h[nr][nc]} < ${h[r][c]}, so water there flows away from here, not into it.`
                },
                [ grid("heights", h).cur([r, c]).done(marked(seen)) ]);
        }
    }

    leave("");
    seen
}

fn blank(rows, cols) {
    let g = [];
    for r in 0..rows {
        let line = [];
        for c in 0..cols { line.push(false); }
        g.push(line);
    }
    g
}

fn trace(input) {
    let h = parse_rows(input.heights);
    let rows = h.len();
    let cols = h[0].len();

    enter("pacificAtlantic", #{ rows: N(rows), cols: N(cols) });

    let pac = blank(rows, cols);
    let atl = blank(rows, cols);
    step("init", "Two marker grids: cells draining to the Pacific (top and left) and to the Atlantic (bottom and right).",
        [ grid("heights", h) ]);

    for r in 0..rows {
        pac = flow(h, pac, r, 0, "Pacific");
        atl = flow(h, atl, r, cols - 1, "Atlantic");
    }
    step("leftRight", "Both side edges done.",
        [ grid("heights", h).hl(marked(pac)).done(marked(atl)) ]);

    for c in 0..cols {
        pac = flow(h, pac, 0, c, "Pacific");
        atl = flow(h, atl, rows - 1, c, "Atlantic");
    }
    step("topBottom", "Top and bottom edges done. Each ocean cost one sweep, not one search per cell.",
        [ grid("heights", h).hl(marked(pac)).done(marked(atl)) ]);

    let res = [];
    let both = [];
    step("collect", "The answer is where the two marked sets overlap.",
        [ grid("heights", h).hl(marked(pac)).done(marked(atl)) ]);

    for r in 0..rows {
        step("rowLoop", `Row ${r}.`, [ grid("heights", h).cur([r, 0]).hl(both) ]);
        for c in 0..cols {
            step("colLoop", `(${r},${c}): Pacific ${pac[r][c]}, Atlantic ${atl[r][c]}.`,
                [ grid("heights", h).cur([r, c]).hl(both) ]);
            if pac[r][c] && atl[r][c] {
                step("both", "Reaches both oceans.",
                    [ grid("heights", h).cur([r, c]).hl(both) ]);
                res.push(`${r},${c}`);
                both.push([r, c]);
                log(`(${r},${c}) reaches both`);
                step("add", `Recorded — ${res.len()} so far.`,
                    [ grid("heights", h).hl(both) ]);
            } else {
                step("both", "Only one ocean at most, so not part of the answer.",
                    [ grid("heights", h).cur([r, c]).hl(both) ]);
            }
        }
    }

    let answer = join(res, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(res.len(), "cell")} drain to both oceans.`,
        [ grid("heights", h).hl(both) ], "return");
    leave(answer);
}
