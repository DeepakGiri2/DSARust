// Triangle — bottom-up, and one row of state is enough.
//
// Going downwards you cannot commit to a direction, because the cheap-looking
// neighbour may lead into an expensive row. Going upwards there is nothing to
// guess: every cell's answer is its value plus the smaller of the two answers
// directly beneath it, and those are already known.
//
// Why one array works: dp[c] is overwritten before dp[c+1] is, and dp[c+1] is
// only ever read as "the row below" — so a left-to-right sweep never reads a
// value it has already replaced.

fn parse_rows(spec) {
    let out = [];
    for part in spec.split(",") {
        let row = [];
        for w in words(part) { row.push(num(w)); }
        if !row.is_empty() { out.push(row); }
    }
    out
}

fn padded(rows, upto) {
    // The triangle as a rectangular grid so the picture keeps its shape.
    let width = rows[rows.len() - 1].len();
    let out = [];
    for r in 0..rows.len() {
        let line = [];
        for c in 0..width {
            line.push(if c < rows[r].len() { `${rows[r][c]}` } else { "" });
        }
        out.push(line);
    }
    out
}

fn trace(input) {
    let rows = parse_rows(input.triangle);
    let n = rows.len();

    enter("minimumTotal", #{ rows: N(n) });

    let dp = [];
    for v in rows[n - 1] { dp.push(v); }
    set(#{ dp: A(dp) });
    step("init", `Seed dp with the last row. From the bottom, the best cost of a cell is just the cell itself — there is nowhere further to go.`,
        [ grid("triangle", padded(rows, n)), array("dp — best cost from this cell down", dp) ]);

    let r = n - 2;
    while r >= 0 {
        set(#{ r: N(r) });
        step("row", `Row ${r}: ${plural(r + 1, "cell")}, each choosing between two cells below.`,
            [ grid("triangle", padded(rows, n)).cur([r, 0]),
              array("dp — best cost from this cell down", dp) ]);

        for c in 0..(r + 1) {
            set(#{ c: N(c) });
            step("col", `Cell (${r}, ${c}) = ${rows[r][c]}. Its two choices below cost ${dp[c]} and ${dp[c + 1]}.`,
                [ grid("triangle", padded(rows, n)).cur([r, c]).hl([[r + 1, c], [r + 1, c + 1]]),
                  array("dp — best cost from this cell down", dp).hl([c, c + 1]) ]);

            let better = if dp[c] < dp[c + 1] { dp[c] } else { dp[c + 1] };
            dp[c] = rows[r][c] + better;
            set(#{ dp: A(dp) });
            step("relax", `dp[${c}] = ${rows[r][c]} + ${better} = ${dp[c]}. The old dp[${c}] is gone, and nothing above still needs it.`,
                [ grid("triangle", padded(rows, n)).cur([r, c]),
                  array("dp — best cost from this cell down", dp).hl([c]) ]);
        }
        r -= 1;
    }

    log(`minimum path sum ${dp[0]}`, "result");
    result(`${dp[0]}`);
    step("ret", `dp[0] = ${dp[0]} — the best cost from the apex, which is the answer.`,
        [ grid("triangle", padded(rows, n)).hl([[0, 0]]),
          array("dp — best cost from this cell down", dp).hl([0]) ],
        "return");
    leave(`${dp[0]}`);
}
