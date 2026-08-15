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

// Edit Distance — three edits, three neighbouring cells.
//
// The table's power is that each of the three possible edits at a position
// corresponds to exactly one neighbour. Replacing consumes a character from
// both words, so it comes from up-left. Deleting consumes one from word1 only:
// from above. Inserting consumes one from word2 only: from the left. A match
// consumes both at no cost, which is why it reads up-left with no +1.

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
    let w1 = input.word1;
    let w2 = input.word2;
    let a = chars(w1);
    let b = chars(w2);
    let m = a.len();
    let n = b.len();

    enter("minDistance", #{ m: N(m), n: N(n) });

    let dp = zeros(m + 1, n + 1);
    step("init", "dp[i][j] is the cheapest way to turn the first i characters of word1 into the first j of word2.",
        [ grid("dp — rows are word1, columns are word2", dp).labels(headers(w1), headers(w2)) ]);

    for i in 0..(m + 1) { grid_set(dp, i, 0, i); }
    step("baseRow", `Turning a prefix of "${w1}" into the empty string means deleting every character, so column 0 counts up.`,
        [ grid("dp — rows are word1, columns are word2", dp).labels(headers(w1), headers(w2))
            .hl(col_cells(m + 1, 0)) ]);

    for j in 0..(n + 1) { grid_set(dp, 0, j, j); }
    step("baseCol", `Building a prefix of "${w2}" from nothing means inserting every character, so row 0 counts up too.`,
        [ grid("dp — rows are word1, columns are word2", dp).labels(headers(w1), headers(w2))
            .hl(row_cells(n + 1, 0)) ]);

    for i in 1..(m + 1) {
        step("rowLoop", `Now the ${i}th character of "${w1}": "${a[i - 1]}".`,
            [ grid("dp — rows are word1, columns are word2", dp).labels(headers(w1), headers(w2))
                .cur([i, 0]) ]);

        for j in 1..(n + 1) {
            step("colLoop", `Against the ${j}th of "${w2}": "${b[j - 1]}".`,
                [ grid("dp — rows are word1, columns are word2", dp)
                    .labels(headers(w1), headers(w2)).cur([i, j]) ]);

            if a[i - 1] == b[j - 1] {
                step("same", `Both are "${a[i - 1]}", so this position needs no edit at all.`,
                    [ grid("dp — rows are word1, columns are word2", dp)
                        .labels(headers(w1), headers(w2)).cur([i, j]).hl([[i - 1, j - 1]]) ]);
                grid_set(dp, i, j, dp[i - 1][j - 1]);
                step("free", `Cost carries over from up-left unchanged: ${dp[i][j]}.`,
                    [ grid("dp — rows are word1, columns are word2", dp)
                        .labels(headers(w1), headers(w2)).cur([i, j]) ]);
                continue;
            }

            step("differ", `"${a[i - 1]}" and "${b[j - 1]}" differ, so one edit has to happen here — and there are exactly three of them.`,
                [ grid("dp — rows are word1, columns are word2", dp)
                    .labels(headers(w1), headers(w2)).cur([i, j])
                    .hl([[i - 1, j - 1], [i - 1, j], [i, j - 1]]) ]);

            let best = dp[i - 1][j - 1];
            step("replace", `Replace "${a[i - 1]}" with "${b[j - 1]}": both words advance, so this reads up-left, ${best}.`,
                [ grid("dp — rows are word1, columns are word2", dp)
                    .labels(headers(w1), headers(w2)).cur([i, j]).hl([[i - 1, j - 1]]) ]);

            if dp[i - 1][j] < best {
                best = dp[i - 1][j];
                step("delete", `Delete "${a[i - 1]}": only word1 advances, so this reads from above, ${best} — cheaper.`,
                    [ grid("dp — rows are word1, columns are word2", dp)
                        .labels(headers(w1), headers(w2)).cur([i, j]).hl([[i - 1, j]]) ]);
            } else {
                step("delete", `Deleting reads from above, ${dp[i - 1][j]}, which is no cheaper.`,
                    [ grid("dp — rows are word1, columns are word2", dp)
                        .labels(headers(w1), headers(w2)).cur([i, j]).hl([[i - 1, j]]) ]);
            }

            if dp[i][j - 1] < best {
                best = dp[i][j - 1];
                step("insert", `Insert "${b[j - 1]}": only word2 advances, so this reads from the left, ${best} — cheaper still.`,
                    [ grid("dp — rows are word1, columns are word2", dp)
                        .labels(headers(w1), headers(w2)).cur([i, j]).hl([[i, j - 1]]) ]);
            } else {
                step("insert", `Inserting reads from the left, ${dp[i][j - 1]}, which is no cheaper.`,
                    [ grid("dp — rows are word1, columns are word2", dp)
                        .labels(headers(w1), headers(w2)).cur([i, j]).hl([[i, j - 1]]) ]);
            }

            grid_set(dp, i, j, best + 1);
            log(`dp[${i}][${j}] = ${dp[i][j]}`);
            step("store", `One edit on top of ${best}: ${dp[i][j]}.`,
                [ grid("dp — rows are word1, columns are word2", dp)
                    .labels(headers(w1), headers(w2)).cur([i, j]) ]);
        }
    }

    let answer = dp[m][n];
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `"${w1}" becomes "${w2}" in ${plural(answer, "edit")}.`,
        [ grid("dp — rows are word1, columns are word2", dp).labels(headers(w1), headers(w2))
            .hl([[m, n]]) ], "return");
    leave(`${answer}`);
}

fn col_cells(rows, c) {
    let out = [];
    for r in 0..rows { out.push([r, c]); }
    out
}

fn row_cells(cols, r) {
    let out = [];
    for c in 0..cols { out.push([r, c]); }
    out
}
