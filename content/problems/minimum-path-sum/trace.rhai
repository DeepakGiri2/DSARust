fn parse_rows(spec) {
    let out = [];
    for row in words(spec) {
        let cells = [];
        for v in row.split(",") { cells.push(num(v)); }
        out.push(cells);
    }
    out
}

// Minimum Path Sum — only the best route into each cell survives.
//
// A cell has at most two predecessors, and the future beyond a cell does not
// depend on how it was reached. That is what makes it safe to throw away every
// route into a cell except the cheapest — the discarded ones could never win
// later either. The first row and column are the degenerate case where there
// is only one predecessor, so no comparison happens at all.

fn zeros(rows, cols) {
    let g = [];
    for r in 0..rows {
        let line = [];
        for c in 0..cols { line.push(0); }
        g.push(line);
    }
    g
}

fn trace(input) {
    let g = parse_rows(input.grid);
    let rows = g.len();
    let cols = g[0].len();

    enter("minPathSum", #{ rows: N(rows), cols: N(cols) });

    let dp = zeros(rows, cols);
    step("init", "dp holds the cheapest total for reaching each cell.",
        [ grid("grid", g), grid("cheapest total into each cell", dp) ]);

    for r in 0..rows {
        step("rowLoop", `Row ${r}.`,
            [ grid("grid", g).cur([r, 0]), grid("cheapest total into each cell", dp) ]);

        for c in 0..cols {
            step("colLoop", `Cell (${r},${c}), costing ${g[r][c]}.`,
                [ grid("grid", g).cur([r, c]).hl([[r, c]]),
                  grid("cheapest total into each cell", dp).cur([r, c]) ]);

            if r == 0 && c == 0 {
                step("start", "The start has no predecessor at all.",
                    [ grid("cheapest total into each cell", dp).cur([r, c]) ]);
                grid_set(dp, r, c, g[r][c]);
                step("seed", `Reaching it costs just its own value, ${g[r][c]}.`,
                    [ grid("cheapest total into each cell", dp).cur([r, c]).hl([[r, c]]) ]);
            } else if r == 0 {
                step("topRow", "On the top row the only way in is from the left — no choice to make.",
                    [ grid("cheapest total into each cell", dp).cur([r, c]).hl([[r, c - 1]]) ]);
                let from = dp[r][c - 1];
                grid_set(dp, r, c, from + g[r][c]);
                step("fromLeft", `${from} + ${g[r][c]} = ${dp[r][c]}.`,
                    [ grid("cheapest total into each cell", dp).cur([r, c]).hl([[r, c]]) ]);
            } else if c == 0 {
                step("leftCol", "On the left column the only way in is from above.",
                    [ grid("cheapest total into each cell", dp).cur([r, c]).hl([[r - 1, c]]) ]);
                grid_set(dp, r, c, dp[r - 1][c] + g[r][c]);
                step("fromAbove", `${dp[r - 1][c]} + ${g[r][c]} = ${dp[r][c]}.`,
                    [ grid("cheapest total into each cell", dp).cur([r, c]).hl([[r, c]]) ]);
            } else {
                step("inner", `Two ways in: from above (${dp[r - 1][c]}) or from the left (${dp[r][c - 1]}).`,
                    [ grid("cheapest total into each cell", dp).cur([r, c])
                        .hl([[r - 1, c], [r, c - 1]]) ]);

                let best = dp[r - 1][c];
                step("pick", `Assume from above: ${best}.`,
                    [ grid("cheapest total into each cell", dp).cur([r, c]).hl([[r - 1, c]]) ]);

                if dp[r][c - 1] < best {
                    best = dp[r][c - 1];
                    step("pickLeft", `From the left is cheaper: ${best}. Whatever route reached that cell, nothing after this point can care which it was.`,
                        [ grid("cheapest total into each cell", dp).cur([r, c]).hl([[r, c - 1]]) ]);
                } else {
                    step("pickLeft", `From the left costs ${dp[r][c - 1]}, which is no better.`,
                        [ grid("cheapest total into each cell", dp).cur([r, c]).hl([[r - 1, c]]) ]);
                }

                grid_set(dp, r, c, best + g[r][c]);
                log(`(${r},${c}) cheapest ${dp[r][c]}`);
                step("store", `${best} + ${g[r][c]} = ${dp[r][c]}.`,
                    [ grid("cheapest total into each cell", dp).cur([r, c]).hl([[r, c]]) ]);
            }
        }
    }

    let answer = dp[rows - 1][cols - 1];
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `The cheapest path costs ${answer}.`,
        [ grid("grid", g),
          grid("cheapest total into each cell", dp).hl([[rows - 1, cols - 1]]) ], "return");
    leave(`${answer}`);
}
