// Game of Life — two generations in one integer.
//
// The naive in-place update is wrong, and subtly: a cell overwritten early
// becomes the wrong input for its neighbour later in the scan. Copying the
// board fixes it at O(m·n) extra memory. Packing fixes it for free — bit 0 is
// the old state, bit 1 is the new one, and every read masks with & 1 so it
// still sees the original board. The final right shift promotes the new
// generation.
//
// The cells show the packed value while the update runs: 1 means "was alive",
// 2 means "will be alive", 3 means both.

fn numeric(g) {
    let out = [];
    for r in 0..g.len() {
        let row = [];
        for c in 0..g[r].len() { row.push(num(g[r][c])); }
        out.push(row);
    }
    out
}

fn neighbour_cells(b, r, c) {
    let out = [];
    for dr in -1..2 {
        for dc in -1..2 {
            if dr == 0 && dc == 0 { continue; }
            let nr = r + dr;
            let nc = c + dc;
            if nr >= 0 && nr < b.len() && nc >= 0 && nc < b[0].len() { out.push([nr, nc]); }
        }
    }
    out
}

fn live_cells(b, cells) {
    let out = [];
    for p in cells {
        if b[p[0]][p[1]] % 2 == 1 { out.push(p); }
    }
    out
}

fn trace(input) {
    let b = numeric(grid_rows(input.board));
    let m = b.len();
    let n = b[0].len();

    enter("gameOfLife", #{ rows: N(m), cols: N(n) });

    for r in 0..m {
        set(#{ r: N(r) });
        step("row", `Row ${r}.`, [ grid("board", b).cur([r, 0]) ]);

        for c in 0..n {
            set(#{ c: N(c) });
            let was_live = b[r][c] % 2 == 1;
            step("col", `Cell (${r}, ${c}) is currently ${if was_live { "alive" } else { "dead" }}.`,
                [ grid("board", b).cur([r, c]) ]);

            step("zero", "Start the neighbour count at zero.",
                [ grid("board", b).cur([r, c]) ]);

            let cells = neighbour_cells(b, r, c);
            step("scan", `Its ${plural(cells.len(), "neighbour")} — the eight around it, clipped at the edges.`,
                [ grid("board", b).cur([r, c]).hl(cells) ]);

            let alive = live_cells(b, cells);
            step("alive", `Reading each with & 1 gives the *old* state, even for cells already updated this pass.`,
                [ grid("board", b).cur([r, c]).hl(alive) ]);

            let live = alive.len();
            set(#{ live: N(live) });
            step("bump", `${plural(live, "live neighbour")}.`,
                [ grid("board", b).cur([r, c]).hl(alive) ]);
            step("total", `neighbours(${r}, ${c}) = ${live}.`,
                [ grid("board", b).cur([r, c]) ]);
            step("count", `live = ${live}.`, [ grid("board", b).cur([r, c]) ]);

            if was_live && (live == 2 || live == 3) {
                step("survive", `Alive with ${live} neighbours — it survives.`,
                    [ grid("board", b).cur([r, c]).hl(alive) ]);
                b[r][c] = b[r][c] | 2;
                step("keep", `Set bit 1: the cell now reads ${b[r][c]} — "was alive, will be alive".`,
                    [ grid("board", b).cur([r, c]) ]);
            } else if !was_live && live == 3 {
                step("born", `Dead with exactly 3 neighbours — a birth.`,
                    [ grid("board", b).cur([r, c]).hl(alive) ]);
                b[r][c] = b[r][c] | 2;
                step("spawn", `Set bit 1: the cell reads ${b[r][c]} — "was dead, will be alive".`,
                    [ grid("board", b).cur([r, c]) ]);
            } else {
                step("survive", `${if was_live { `Alive with ${live} neighbours — ${if live < 2 { "too few" } else { "too many" }}, it dies.` } else { `Dead with ${live} neighbours — no birth.` }} Bit 1 stays 0, which already means "will be dead".`,
                    [ grid("board", b).cur([r, c]).bad([[r, c]]) ]);
            }
        }
    }

    step("second", "Every cell now carries both generations. One more pass reveals the new one.",
        [ grid("board", b) ]);

    for r in 0..m {
        for c in 0..n {
            b[r][c] = b[r][c] / 2;
        }
        step("shift", `Row ${r} shifted right by one — bit 1 becomes bit 0.`,
            [ grid("board", b).done([[r, 0]]) ]);
    }

    let rows = [];
    for r in 0..m { rows.push(join(b[r], "")); }
    let out = join(rows, " ");
    log(out, "result");
    result(out);
    step("second", `The next generation, computed in place: ${out}.`,
        [ grid("board", b) ], "return");
    leave(out);
}
