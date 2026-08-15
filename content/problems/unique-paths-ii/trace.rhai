// Unique Paths II — an obstacle is a zero, not a rule.
//
// Paths arriving from above and paths arriving from the left are disjoint sets
// — a path enters a cell through exactly one of them — so the counts simply
// add. Blocking then needs no separate handling: an obstacle holds 0, and
// every cell downstream inherits that 0 through the same addition, so whole
// regions become unreachable without any explicit propagation.

fn zeros(rows, cols) {
    let g = [];
    for r in 0..rows {
        let line = [];
        for c in 0..cols { line.push(0); }
        g.push(line);
    }
    g
}

fn walls(g) {
    let out = [];
    for r in 0..g.len() {
        for c in 0..g[0].len() {
            if g[r][c] == "1" { out.push([r, c]); }
        }
    }
    out
}

fn trace(input) {
    let g = grid_rows(input.grid);
    let rows = g.len();
    let cols = g[0].len();

    enter("uniquePathsWithObstacles", #{ rows: N(rows), cols: N(cols) });

    let dp = zeros(rows, cols);
    step("init", "One count per cell: how many right/down paths reach it from the start.",
        [ grid("obstacles", g).bad(walls(g)), grid("paths into each cell", dp) ]);

    for r in 0..rows {
        step("rowLoop", `Row ${r}.`,
            [ grid("obstacles", g).bad(walls(g)),
              grid("paths into each cell", dp).cur([r, 0]) ]);

        for c in 0..cols {
            step("colLoop", `Cell (${r},${c}).`,
                [ grid("obstacles", g).cur([r, c]).bad(walls(g)),
                  grid("paths into each cell", dp).cur([r, c]) ]);

            if g[r][c] == "1" {
                step("blocked", `(${r},${c}) is an obstacle.`,
                    [ grid("obstacles", g).cur([r, c]).bad(walls(g)) ]);
                grid_set(dp, r, c, 0);
                step("zero", "It holds 0 paths — and because later cells only ever add their neighbours, that 0 spreads on its own.",
                    [ grid("paths into each cell", dp).cur([r, c]).bad([[r, c]]) ]);
                continue;
            }

            if r == 0 && c == 0 {
                step("start", "This is the start.",
                    [ grid("paths into each cell", dp).cur([r, c]) ]);
                grid_set(dp, r, c, 1);
                step("seed", "There is exactly one way to be standing where you began: do nothing.",
                    [ grid("paths into each cell", dp).cur([r, c]).hl([[r, c]]) ]);
                continue;
            }

            let total = 0;
            step("sum", "Count the paths arriving here.",
                [ grid("paths into each cell", dp).cur([r, c]) ]);

            if r > 0 {
                step("fromAbove", `The cell above holds ${dp[r - 1][c]} path(s).`,
                    [ grid("paths into each cell", dp).cur([r, c]).hl([[r - 1, c]]) ]);
                total += dp[r - 1][c];
                step("addAbove", `Running total ${total}.`,
                    [ grid("paths into each cell", dp).cur([r, c]).hl([[r - 1, c]]) ]);
            } else {
                step("fromAbove", "There is no row above, so nothing arrives from that direction.",
                    [ grid("paths into each cell", dp).cur([r, c]) ]);
            }

            if c > 0 {
                step("fromLeft", `The cell to the left holds ${dp[r][c - 1]} path(s). No path is counted twice, because a path enters through exactly one side.`,
                    [ grid("paths into each cell", dp).cur([r, c]).hl([[r, c - 1]]) ]);
                total += dp[r][c - 1];
                step("addLeft", `Running total ${total}.`,
                    [ grid("paths into each cell", dp).cur([r, c]).hl([[r, c - 1]]) ]);
            } else {
                step("fromLeft", "There is no column to the left.",
                    [ grid("paths into each cell", dp).cur([r, c]) ]);
            }

            grid_set(dp, r, c, total);
            step("store", `(${r},${c}) is reachable by ${plural(total, "path")}.`,
                [ grid("paths into each cell", dp).cur([r, c]).hl([[r, c]]) ]);
        }
    }

    let answer = dp[rows - 1][cols - 1];
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `${plural(answer, "path")} reach the bottom-right corner.`,
        [ grid("obstacles", g).bad(walls(g)),
          grid("paths into each cell", dp).hl([[rows - 1, cols - 1]]) ], "return");
    leave(`${answer}`);
}
