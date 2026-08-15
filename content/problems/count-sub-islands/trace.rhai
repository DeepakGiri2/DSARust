// Count Sub Islands — do not stop early, even once the answer is known.
//
// A flood fill that returns a verdict. The tempting optimisation — bail out as
// soon as one cell fails — is a real bug: the rest of the island would stay
// unsunk and the outer scan would find it again, counting one island several
// times. So every neighbour is explored whatever the verdict, and failure is
// merged into the result rather than propagated by returning early.

fn sunk_cells(g2, orig) {
    let out = [];
    for r in 0..g2.len() {
        for c in 0..g2[0].len() {
            if orig[r][c] == "1" && g2[r][c] == "0" { out.push([r, c]); }
        }
    }
    out
}

fn sink(g1, g2, r, c, orig) {
    if r < 0 || r >= g2.len() || c < 0 || c >= g2[0].len() || g2[r][c] == "0" {
        enter(`sink(${r},${c})`);
        step("bounds", `(${r},${c}) is off the grid or already sunk — nothing here can spoil the island.`,
            [ grid("grid2 (being sunk)", g2).done(sunk_cells(g2, orig)) ], "return");
        leave("true");
        return [g2, true];
    }

    enter(`sink(${r},${c})`, #{ r: N(r), c: N(c) });
    step("bounds", `(${r},${c}) is part of the island.`,
        [ grid("grid1 (reference)", g1).hl([[r, c]]),
          grid("grid2 (being sunk)", g2).cur([r, c]).done(sunk_cells(g2, orig)) ]);

    grid_set(g2, r, c, "0");
    step("visit", "Sink it so the outer scan cannot start here again.",
        [ grid("grid2 (being sunk)", g2).cur([r, c]).done(sunk_cells(g2, orig)) ]);

    let ok = g1[r][c] == "1";
    step("overlap",
        if ok { `grid1 also has land at (${r},${c}), so this cell is fine.` }
        else { `grid1 has water at (${r},${c}) — this island cannot be a sub-island.` },
        [ grid("grid1 (reference)", g1).cur([r, c]).hl(if ok { [[r, c]] } else { [] })
              .bad(if ok { [] } else { [[r, c]] }),
          grid("grid2 (being sunk)", g2).cur([r, c]).done(sunk_cells(g2, orig)) ]);

    let dirs = [[1, 0], [-1, 0], [0, 1], [0, -1]];
    for d in dirs {
        step("neighbours", `Continuing to (${r + d[0]},${c + d[1]}).`,
            [ grid("grid2 (being sunk)", g2).cur([r, c]).done(sunk_cells(g2, orig)) ]);
        let out = sink(g1, g2, r + d[0], c + d[1], orig);
        g2 = out[0];
        if !out[1] {
            step("recurse", "That direction reported a failure.",
                [ grid("grid2 (being sunk)", g2).done(sunk_cells(g2, orig)) ]);
            ok = false;
            step("fail", "Record it — but keep exploring, because the whole island still has to be sunk or it will be counted again.",
                [ grid("grid2 (being sunk)", g2).done(sunk_cells(g2, orig)) ]);
        } else {
            step("recurse", "That direction is clear so far.",
                [ grid("grid2 (being sunk)", g2).done(sunk_cells(g2, orig)) ]);
        }
    }

    step("verdict", `Subtree verdict from (${r},${c}): ${ok}.`,
        [ grid("grid2 (being sunk)", g2).done(sunk_cells(g2, orig)) ], "return");
    leave(`${ok}`);
    [g2, ok]
}

fn trace(input) {
    let g1 = grid_rows(input.grid1);
    let g2 = grid_rows(input.grid2);
    let orig = grid_rows(input.grid2);
    let rows = g2.len();
    let cols = g2[0].len();

    enter("countSubIslands", #{ rows: N(rows), cols: N(cols) });

    let count = 0;
    set(#{ count: N(count) });
    step("init", "Count islands of grid2 that lie entirely inside grid1's land.",
        [ grid("grid1 (reference)", g1), grid("grid2 (being sunk)", g2) ]);

    for r in 0..rows {
        step("rowLoop", `Row ${r}.`,
            [ grid("grid2 (being sunk)", g2).cur([r, 0]).done(sunk_cells(g2, orig)) ]);

        for c in 0..cols {
            step("colLoop", `(${r},${c}).`,
                [ grid("grid2 (being sunk)", g2).cur([r, c]).done(sunk_cells(g2, orig)) ]);

            if g2[r][c] == "1" {
                step("land", "Unvisited land — the start of a new island.",
                    [ grid("grid2 (being sunk)", g2).cur([r, c]).hl([[r, c]]) ]);
                step("check", "Sink the whole island and find out whether every cell overlapped.",
                    [ grid("grid2 (being sunk)", g2).cur([r, c]) ], "call");
                let out = sink(g1, g2, r, c, orig);
                g2 = out[0];
                if out[1] {
                    count += 1;
                    set(#{ count: N(count) });
                    log(`sub-island found; count ${count}`);
                    step("bump", `Every cell overlapped, so that is sub-island number ${count}.`,
                        [ grid("grid2 (being sunk)", g2).done(sunk_cells(g2, orig)) ]);
                } else {
                    step("bump", `At least one cell had no matching land, so it does not count. Total still ${count}.`,
                        [ grid("grid2 (being sunk)", g2).done(sunk_cells(g2, orig)) ]);
                }
            }
        }
    }

    log(`answer ${count}`, "result");
    result(`${count}`);
    step("ret", `${plural(count, "sub-island")}.`,
        [ grid("grid1 (reference)", g1), grid("grid2 (being sunk)", g2) ], "return");
    leave(`${count}`);
}
