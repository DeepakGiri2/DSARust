// Island Perimeter — count edges, not islands.
//
// The word "island" suggests a traversal, but connectivity is irrelevant: the
// perimeter is a local property. Four edges per land cell, minus two for every
// adjacency. Looking only up and left is what stops each adjacency being
// subtracted twice — every pair has exactly one member that sees the other in
// those two directions.

fn land_cells(g) {
    let out = [];
    for r in 0..g.len() {
        for c in 0..g[0].len() { if g[r][c] == "1" { out.push([r, c]); } }
    }
    out
}

fn trace(input) {
    let g = grid_rows(input.grid);
    let rows = g.len();
    let cols = g[0].len();

    enter("islandPerimeter", #{ rows: N(rows), cols: N(cols) });

    let total = 0;
    set(#{ total: N(total) });
    step("init", "One pass, no traversal — perimeter is decided cell by cell.",
        [ grid("grid", g).hl(land_cells(g)) ]);

    for r in 0..rows {
        step("rowLoop", `Row ${r}.`, [ grid("grid", g).cur([r, 0]).hl(land_cells(g)) ]);

        for c in 0..cols {
            step("colLoop", `Cell (${r},${c}).`,
                [ grid("grid", g).cur([r, c]).hl(land_cells(g)),
                  text("perimeter", [`${total}`]) ]);

            if g[r][c] == "0" {
                step("water", "Water contributes nothing.",
                    [ grid("grid", g).cur([r, c]).hl(land_cells(g)) ]);
                continue;
            }

            total += 4;
            set(#{ total: N(total) });
            step("four", `A land cell on its own has four exposed edges: ${total}.`,
                [ grid("grid", g).cur([r, c]).hl([[r, c]]),
                  text("perimeter", [`${total}`]) ]);

            if r > 0 && g[r - 1][c] == "1" {
                step("up", `The cell above is land too, so those two faces touch.`,
                    [ grid("grid", g).cur([r, c]).hl([[r, c], [r - 1, c]]) ]);
                total -= 2;
                set(#{ total: N(total) });
                log(`shared edge with (${r - 1},${c})`);
                step("shareUp", `Neither face is on the perimeter: ${total}.`,
                    [ text("perimeter", [`${total}`]) ]);
            } else {
                step("up", "Nothing directly above, so that face is exposed.",
                    [ grid("grid", g).cur([r, c]).hl([[r, c]]) ]);
            }

            if c > 0 && g[r][c - 1] == "1" {
                step("left", `The cell to the left is land. Checking only up and left is what keeps every adjacency from being counted twice.`,
                    [ grid("grid", g).cur([r, c]).hl([[r, c], [r, c - 1]]) ]);
                total -= 2;
                set(#{ total: N(total) });
                log(`shared edge with (${r},${c - 1})`);
                step("shareLeft", `Two more faces hidden: ${total}.`,
                    [ text("perimeter", [`${total}`]) ]);
            } else {
                step("left", "Nothing directly to the left.",
                    [ grid("grid", g).cur([r, c]).hl([[r, c]]) ]);
            }
        }
    }

    log(`answer ${total}`, "result");
    result(`${total}`);
    step("ret", `The island's perimeter is ${total}.`,
        [ grid("grid", g).hl(land_cells(g)), text("perimeter", [`${total}`]) ], "return");
    leave(`${total}`);
}
