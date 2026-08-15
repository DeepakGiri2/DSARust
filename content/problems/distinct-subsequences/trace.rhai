fn headers(s) {
    let out = ["-"];
    for c in chars(s) { out.push(c); }
    out
}

fn bool_table(dp) {
    let out = [];
    for row in dp {
        let line = [];
        for v in row { line.push(if v { "T" } else { "·" }); }
        out.push(line);
    }
    out
}

// Distinct Subsequences — skipping is free, matching is a choice.
//
// Every cell starts by inheriting the row above: whatever could be built
// without this character of s can still be built. A match then *adds* — it
// does not replace — because using the character is a genuinely different
// subsequence from not using it. That addition is what makes the count
// distinguish "bag" taken from positions 0,1,3 from the same letters taken
// from 0,4,6.

fn zeros(rows, cols) {
    let g = [];
    for r in 0..rows {
        let line = [];
        for c in 0..cols { line.push(0); }
        g.push(line);
    }
    g
}

fn col_cells(rows, c) {
    let out = [];
    for r in 0..rows { out.push([r, c]); }
    out
}

fn trace(input) {
    let s = input.s;
    let t = input.t;
    let a = chars(s);
    let b = chars(t);
    let m = a.len();
    let n = b.len();

    enter("numDistinct", #{ m: N(m), n: N(n) });

    let dp = zeros(m + 1, n + 1);
    step("init", "dp[i][j] counts the ways the first i characters of s can produce the first j of t.",
        [ grid("dp — rows are s, columns are t", dp).labels(headers(s), headers(t)) ]);

    for i in 0..(m + 1) { grid_set(dp, i, 0, 1); }
    step("base", "Any prefix of s produces the empty target exactly one way: take nothing.",
        [ grid("dp — rows are s, columns are t", dp).labels(headers(s), headers(t))
            .hl(col_cells(m + 1, 0)) ]);

    for i in 1..(m + 1) {
        step("rowLoop", `Bringing in s[${i - 1}] = "${a[i - 1]}".`,
            [ grid("dp — rows are s, columns are t", dp).labels(headers(s), headers(t))
                .cur([i, 0]) ]);

        for j in 1..(n + 1) {
            step("colLoop", `Matching against t[${j - 1}] = "${b[j - 1]}".`,
                [ grid("dp — rows are s, columns are t", dp).labels(headers(s), headers(t))
                    .cur([i, j]) ]);

            grid_set(dp, i, j, dp[i - 1][j]);
            step("skip", `Ignoring "${a[i - 1]}" is always allowed, so everything the row above could do still counts: ${dp[i][j]}.`,
                [ grid("dp — rows are s, columns are t", dp).labels(headers(s), headers(t))
                    .cur([i, j]).hl([[i - 1, j]]) ]);

            if a[i - 1] == b[j - 1] {
                step("match", `"${a[i - 1]}" matches "${b[j - 1]}", so it may also be *used* — a different subsequence from ignoring it.`,
                    [ grid("dp — rows are s, columns are t", dp).labels(headers(s), headers(t))
                        .cur([i, j]).hl([[i - 1, j - 1]]) ]);
                grid_set(dp, i, j, dp[i][j] + dp[i - 1][j - 1]);
                log(`dp[${i}][${j}] = ${dp[i][j]}`);
                step("use", `Add the ${dp[i - 1][j - 1]} way(s) of having matched the rest: ${dp[i][j]}.`,
                    [ grid("dp — rows are s, columns are t", dp).labels(headers(s), headers(t))
                        .cur([i, j]) ]);
            } else {
                step("match", `"${a[i - 1]}" is not "${b[j - 1]}", so it cannot be used here — only ignored.`,
                    [ grid("dp — rows are s, columns are t", dp).labels(headers(s), headers(t))
                        .cur([i, j]) ]);
            }
        }
    }

    let answer = dp[m][n];
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `"${t}" appears as a subsequence of "${s}" in ${plural(answer, "distinct way")}.`,
        [ grid("dp — rows are s, columns are t", dp).labels(headers(s), headers(t))
            .hl([[m, n]]) ], "return");
    leave(`${answer}`);
}
