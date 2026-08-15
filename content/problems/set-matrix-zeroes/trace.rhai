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

// Set Matrix Zeroes — mark first, apply second, and borrow the margins.
//
// Zeroing in place as you scan is self-defeating: the zeros you write look
// exactly like the zeros you were told about. So the two phases must be
// separated, and the marks need storage. The first row and column are the one
// place that costs nothing, because if anything in them is zero they are
// doomed anyway. Their intersection is the single ambiguous cell, which is why
// the first row's original state is kept in a flag and both margins are
// written last.

fn first_row_cells(cols) {
    let out = [];
    for c in 0..cols { out.push([0, c]); }
    out
}

fn first_col_cells(rows) {
    let out = [];
    for r in 0..rows { out.push([r, 0]); }
    out
}

fn margin_cells(rows, cols) {
    let out = first_row_cells(cols);
    for r in 1..rows { out.push([r, 0]); }
    out
}

fn trace(input) {
    let g = parse_rows(input.matrix);
    let rows = g.len();
    let cols = g[0].len();

    enter("setZeroes", #{ rows: N(rows), cols: N(cols) });

    let first_row = false;
    set(#{ firstRow: B(first_row) });
    step("flag", "One flag records whether the first row already contained a zero. Everything else can be recorded inside the matrix itself.",
        [ grid("matrix", g) ]);

    for c in 0..cols {
        if g[0][c] == 0 { first_row = true; }
    }
    set(#{ firstRow: B(first_row) });
    step("scanFirst",
        if first_row {
            "The first row does contain a zero, so it is doomed — which is exactly why it is safe to scribble on."
        } else {
            "The first row contains no zero of its own, so any zero appearing there later must be a mark.".to_string()
        },
        [ grid("matrix", g).hl(first_row_cells(cols)) ]);

    for r in 1..rows {
        step("markRows", `Scanning row ${r} for zeros.`,
            [ grid("matrix", g).hl(margin_cells(rows, cols)) ]);
        for c in 0..cols {
            step("markCols", `Cell (${r},${c}) holds ${g[r][c]}.`,
                [ grid("matrix", g).cur([r, c]).hl(margin_cells(rows, cols)) ]);
            if g[r][c] == 0 {
                step("found", `A zero at (${r},${c}) — row ${r} and column ${c} both have to go.`,
                    [ grid("matrix", g).cur([r, c]).bad([[r, c]]) ]);
                grid_set(g, 0, c, 0);
                log(`mark column ${c}`);
                step("markCol", `Note it at the top of column ${c} rather than in a separate array.`,
                    [ grid("matrix", g).cur([0, c]).hl(margin_cells(rows, cols)) ]);
                grid_set(g, r, 0, 0);
                log(`mark row ${r}`);
                step("markRow", `And at the start of row ${r}.`,
                    [ grid("matrix", g).cur([r, 0]).hl(margin_cells(rows, cols)) ]);
            }
        }
    }

    for r in 1..rows {
        step("applyRows", `Applying the marks to row ${r}.`,
            [ grid("matrix", g).hl(margin_cells(rows, cols)) ]);
        for c in 1..cols {
            step("applyCols", `Cell (${r},${c}).`,
                [ grid("matrix", g).cur([r, c]).hl([[0, c], [r, 0]]) ]);
            if g[0][c] == 0 || g[r][0] == 0 {
                step("check", `Its column header is ${g[0][c]} and its row header is ${g[r][0]} — at least one is a mark.`,
                    [ grid("matrix", g).cur([r, c]).hl([[0, c], [r, 0]]) ]);
                grid_set(g, r, c, 0);
                step("zero", "So this cell becomes 0. The margins are read, never rewritten, during this phase.",
                    [ grid("matrix", g).cur([r, c]).bad([[r, c]]) ]);
            } else {
                step("check", `Neither header is a mark, so (${r},${c}) survives.`,
                    [ grid("matrix", g).cur([r, c]).hl([[0, c], [r, 0]]) ]);
            }
        }
    }

    if g[0][0] == 0 {
        step("colZero", "The corner is 0, which means some cell in the first column was zero — so the whole column goes.",
            [ grid("matrix", g).hl(first_col_cells(rows)) ]);
        for r in 0..rows { grid_set(g, r, 0, 0); }
        step("wipeCol", "First column zeroed. Doing this only now is what kept the marks readable.",
            [ grid("matrix", g).bad(first_col_cells(rows)) ]);
    } else {
        step("colZero", "The corner is not 0, so the first column had no zero of its own.",
            [ grid("matrix", g).hl(first_col_cells(rows)) ]);
    }

    if first_row {
        step("rowZero", "The saved flag says the first row originally held a zero.",
            [ grid("matrix", g).hl(first_row_cells(cols)) ]);
        for c in 0..cols { grid_set(g, 0, c, 0); }
        step("wipeRow", "First row zeroed — last, because until now it was the notepad.",
            [ grid("matrix", g).bad(first_row_cells(cols)) ]);
    } else {
        step("rowZero", "The flag says the first row had no zero of its own, so it keeps whatever survived.",
            [ grid("matrix", g).hl(first_row_cells(cols)) ]);
        step("wipeRow", "Nothing to wipe.", [ grid("matrix", g) ]);
    }

    let answer = render_rows(g);
    log(`answer ${answer}`, "result");
    result(answer);
    leave(answer);
}
