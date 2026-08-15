// Range Sum Query 2D — inclusion–exclusion, twice.
//
// Building the table and answering a query are the same identity in opposite
// directions. Building: this cell = value + the strip above + the strip left −
// the corner both strips contain. Querying: the rectangle = everything up to
// the far corner − the strip above it − the strip left of it + the corner that
// got subtracted twice. Watch the highlighted cells and the ± signs match up.

fn parse_queries(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 4 { out.push([num(w[0]), num(w[1]), num(w[2]), num(w[3])]); }
    }
    out
}

fn numeric(g) {
    let out = [];
    for r in 0..g.len() {
        let row = [];
        for c in 0..g[r].len() { row.push(num(g[r][c])); }
        out.push(row);
    }
    out
}

fn zeros(rows, cols) {
    let out = [];
    for r in 0..rows {
        let row = [];
        for c in 0..cols { row.push(0); }
        out.push(row);
    }
    out
}

fn trace(input) {
    let matrix = numeric(grid_rows(input.matrix));
    let m = matrix.len();
    let n = matrix[0].len();
    let queries = parse_queries(input.queries);

    enter("Constructor", #{ rows: N(m), cols: N(n) });

    let pre = zeros(m + 1, n + 1);
    set(#{ pre: S("(m+1) × (n+1) zeros") });
    step("init",
        `pre is ${m + 1} × ${n + 1} — one extra row and column of zeros along the top and left, so the formula never reads off the edge.`,
        [ grid("matrix", matrix), grid("pre — sums from the origin", pre) ]);

    for r in 0..m {
        step("row", `Row ${r} of the matrix.`,
            [ grid("matrix", matrix).cur([r, 0]), grid("pre — sums from the origin", pre) ]);
        for c in 0..n {
            step("col", `Cell (${r}, ${c}) = ${matrix[r][c]}.`,
                [ grid("matrix", matrix).cur([r, c]),
                  grid("pre — sums from the origin", pre).hl([[r, c + 1], [r + 1, c], [r, c]]) ]);

            let above = pre[r][c + 1];
            let left = pre[r + 1][c];
            let corner = pre[r][c];
            pre[r + 1][c + 1] = matrix[r][c] + above + left - corner;
            set(#{ above: N(above), left: N(left), overlap: N(corner) });
            step("cell",
                `pre[${r + 1}][${c + 1}] = ${matrix[r][c]} + ${above} (above) + ${left} (left) − ${corner} (counted twice) = ${pre[r + 1][c + 1]}.`,
                [ grid("matrix", matrix).cur([r, c]),
                  grid("pre — sums from the origin", pre).cur([r + 1, c + 1]).hl([[r, c + 1], [r + 1, c], [r, c]]) ]);
        }
    }

    step("ready", "The table is built. Every rectangle from here costs four lookups, whatever its size.",
        [ grid("matrix", matrix), grid("pre — sums from the origin", pre) ], "return");
    leave("built");

    let answers = [];
    for q in 0..queries.len() {
        let r1 = queries[q][0];
        let c1 = queries[q][1];
        let r2 = queries[q][2];
        let c2 = queries[q][3];

        let region = [];
        for r in r1..(r2 + 1) {
            for c in c1..(c2 + 1) { region.push([r, c]); }
        }

        enter(`sumRegion(${r1},${c1},${r2},${c2})`, #{ r1: N(r1), c1: N(c1), r2: N(r2), c2: N(c2) });
        step("query", `Query ${q}: the rectangle from (${r1}, ${c1}) to (${r2}, ${c2}).`,
            [ grid("matrix", matrix).hl(region),
              grid("pre — sums from the origin", pre) ]);

        let big = pre[r2 + 1][c2 + 1];
        let top = pre[r1][c2 + 1];
        let side = pre[r2 + 1][c1];
        let back = pre[r1][c1];
        let ans = big - top - side + back;
        set(#{ whole: N(big), top: N(top), left: N(side), corner: N(back) });
        log(`sumRegion(${r1},${c1},${r2},${c2}) = ${ans}`, "result");
        step("answer",
            `${big} (everything up to the far corner) − ${top} (the strip above) − ${side} (the strip to the left) + ${back} (removed twice, add it back) = ${ans}.`,
            [ grid("matrix", matrix).hl(region),
              grid("pre — sums from the origin", pre)
                  .cur([r2 + 1, c2 + 1]).hl([[r1, c2 + 1], [r2 + 1, c1]]).done([[r1, c1]]) ],
            "return");
        leave(`${ans}`);
        answers.push(ans);
    }

    result(join(answers, " "));
}
