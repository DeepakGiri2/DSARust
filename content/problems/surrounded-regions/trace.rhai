// Surrounded Regions — search for the survivors, not the victims.
//
// "Is this region enclosed?" is awkward: it can only be answered after the
// whole region has been walked without ever touching an edge. The complement
// is easy — a region survives exactly when it is reachable from the border. So
// flood inwards from every edge cell, marking survivors with a temporary
// letter, and let the final sweep decide by what is still unmarked.

fn render(g) {
    let out = [];
    for row in g { out.push(join(row, "")); }
    join(out, " ")
}

fn saved_cells(g) {
    let out = [];
    for r in 0..g.len() {
        for c in 0..g[0].len() { if g[r][c] == "S" { out.push([r, c]); } }
    }
    out
}

fn mark(g, r, c) {
    if r < 0 || r >= g.len() || c < 0 || c >= g[0].len() || g[r][c] != "O" {
        enter(`mark(${r},${c})`);
        step("bounds", `(${r},${c}) is off the board, or not an unvisited O.`,
            [ grid("board", g).done(saved_cells(g)) ], "return");
        leave("");
        return g;
    }

    enter(`mark(${r},${c})`, #{ r: N(r), c: N(c) });
    step("bounds", `(${r},${c}) is an O connected to the border.`,
        [ grid("board", g).cur([r, c]).done(saved_cells(g)) ]);

    grid_set(g, r, c, "S");
    log(`save (${r},${c})`);
    step("save", "Mark it S — safe. The letter doubles as the visited flag.",
        [ grid("board", g).cur([r, c]).done(saved_cells(g)) ]);

    g = mark(g, r + 1, c);
    step("down", "Down explored.", [ grid("board", g).cur([r, c]).done(saved_cells(g)) ]);
    g = mark(g, r - 1, c);
    step("up", "Up explored.", [ grid("board", g).cur([r, c]).done(saved_cells(g)) ]);
    g = mark(g, r, c + 1);
    step("right", "Right explored.", [ grid("board", g).cur([r, c]).done(saved_cells(g)) ]);
    g = mark(g, r, c - 1);
    step("left", "Left explored.", [ grid("board", g).done(saved_cells(g)) ], "return");

    leave("");
    g
}

fn trace(input) {
    let g = grid_rows(input.board);
    let rows = g.len();
    let cols = g[0].len();

    enter("solve", #{ rows: N(rows), cols: N(cols) });

    for r in 0..rows {
        g = mark(g, r, 0);
        g = mark(g, r, cols - 1);
    }
    step("leftRight", "Flood inwards from every cell of the left and right edges.",
        [ grid("board", g).done(saved_cells(g)) ]);

    for c in 0..cols {
        g = mark(g, 0, c);
        g = mark(g, rows - 1, c);
    }
    step("topBottom", `Now the top and bottom edges. Everything marked S is provably not surrounded, because a path of Os leads it to an edge.`,
        [ grid("board", g).done(saved_cells(g)) ]);

    for r in 0..rows {
        step("rowLoop", `Sweeping row ${r}.`,
            [ grid("board", g).cur([r, 0]).done(saved_cells(g)) ]);

        for c in 0..cols {
            step("colLoop", `Cell (${r},${c}) holds ${g[r][c]}.`,
                [ grid("board", g).cur([r, c]).done(saved_cells(g)) ]);

            if g[r][c] == "O" {
                step("captured", `Still an O, which means the border flood never reached it — so it is surrounded.`,
                    [ grid("board", g).cur([r, c]).bad([[r, c]]) ]);
                grid_set(g, r, c, "X");
                step("flip", "Capture it.", [ grid("board", g).cur([r, c]) ]);
            } else if g[r][c] == "S" {
                step("safe", "Marked safe during the flood.",
                    [ grid("board", g).cur([r, c]).done(saved_cells(g)) ]);
                grid_set(g, r, c, "O");
                step("restore", "Restore it to O.", [ grid("board", g).cur([r, c]) ]);
            } else {
                step("captured", "An X — nothing to decide.",
                    [ grid("board", g).cur([r, c]) ]);
                step("safe", "Leave it alone.", [ grid("board", g).cur([r, c]) ]);
            }
        }
    }

    let answer = render(g);
    log(`answer ${answer}`, "result");
    result(answer);
    leave(answer);
}
