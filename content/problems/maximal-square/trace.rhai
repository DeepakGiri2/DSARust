// Maximal Square — the minimum of three neighbours is the geometry.
//
// Anchoring each square at its bottom-right corner turns a two-dimensional
// search into one number per cell. A square of side k ending here requires
// squares of side k-1 ending directly above, directly left, and diagonally
// up-left; if any of them is smaller, the square cannot be that big. Hence the
// minimum — one short neighbour is enough to cap it.

fn zeros(rows, cols) {
    let g = [];
    for r in 0..rows {
        let line = [];
        for c in 0..cols { line.push(0); }
        g.push(line);
    }
    g
}

fn min3(a, b, c) {
    let m = if a < b { a } else { b };
    if c < m { c } else { m }
}

fn square_cells(r, c, side) {
    let out = [];
    let i = r - side + 1;
    while i <= r {
        let j = c - side + 1;
        while j <= c { out.push([i, j]); j += 1; }
        i += 1;
    }
    out
}

fn trace(input) {
    let m = grid_rows(input.matrix);
    let rows = m.len();
    let cols = m[0].len();

    enter("maximalSquare", #{ rows: N(rows), cols: N(cols) });

    // The dp table is padded by one row and column so the neighbour lookups
    // never need a bounds check.
    let dp = zeros(rows + 1, cols + 1);
    step("init", "dp[r][c] is the side of the largest all-ones square whose bottom-right corner is that cell. A padding row and column of zeros removes every bounds check.",
        [ grid("matrix", m), grid("largest square ending here (side)", dp) ]);

    let best = 0;
    let best_cells = [];
    set(#{ best: N(best) });
    step("best", "No square found yet.",
        [ grid("matrix", m), grid("largest square ending here (side)", dp) ]);

    for r in 1..(rows + 1) {
        step("rowLoop", `Row ${r - 1} of the matrix.`,
            [ grid("matrix", m).cur([r - 1, 0]),
              grid("largest square ending here (side)", dp) ]);

        for c in 1..(cols + 1) {
            step("colLoop", `Cell (${r - 1},${c - 1}) holds ${m[r - 1][c - 1]}.`,
                [ grid("matrix", m).cur([r - 1, c - 1]),
                  grid("largest square ending here (side)", dp).cur([r, c]) ]);

            if m[r - 1][c - 1] == "0" {
                step("zero", "A 0 can be the corner of nothing, so its entry stays 0.",
                    [ grid("matrix", m).cur([r - 1, c - 1]).bad([[r - 1, c - 1]]),
                      grid("largest square ending here (side)", dp).cur([r, c]) ]);
                continue;
            }

            let side = min3(dp[r - 1][c], dp[r][c - 1], dp[r - 1][c - 1]);
            step("side", `Above ${dp[r - 1][c]}, left ${dp[r][c - 1]}, diagonal ${dp[r - 1][c - 1]}. The smallest is ${side} — any of the three being short caps the square here.`,
                [ grid("largest square ending here (side)", dp).cur([r, c])
                    .hl([[r - 1, c], [r, c - 1], [r - 1, c - 1]]) ]);

            grid_set(dp, r, c, side + 1);
            step("store", `So the largest square cornered at (${r - 1},${c - 1}) has side ${side + 1}.`,
                [ grid("matrix", m).cur([r - 1, c - 1])
                    .hl(square_cells(r - 1, c - 1, side + 1)),
                  grid("largest square ending here (side)", dp).cur([r, c]) ]);

            if dp[r][c] > best {
                step("improve", `${dp[r][c]} beats the best side so far, ${best}.`,
                    [ grid("largest square ending here (side)", dp).cur([r, c]) ]);
                best = dp[r][c];
                best_cells = square_cells(r - 1, c - 1, best);
                set(#{ best: N(best) });
                log(`side ${best} at (${r - 1},${c - 1})`);
                step("update", `Best side is now ${best}.`,
                    [ grid("matrix", m).hl(best_cells) ]);
            } else {
                step("improve", `${dp[r][c]} does not beat ${best}.`,
                    [ grid("largest square ending here (side)", dp).cur([r, c]) ]);
            }
        }
    }

    let area = best * best;
    log(`answer ${area}`, "result");
    result(`${area}`);
    step("ret", `The largest square has side ${best}, so its area is ${best} × ${best} = ${area}.`,
        [ grid("matrix", m).hl(best_cells) ], "return");
    leave(`${area}`);
}
