// Longest Common Subsequence — a table over prefixes of both strings.
//
// dp[i][j] answers "the best I can do using the first i letters of text1 and
// the first j of text2". Matching letters extend the diagonal; otherwise one
// of the two letters has to be given up, and the better choice wins.

fn as_strings(grid) {
    let out = [];
    for row in grid {
        let line = [];
        for v in row { line.push(`${v}`); }
        out.push(line);
    }
    out
}

fn labels_for(cs) {
    let out = ["-"];
    for c in cs { out.push(c); }
    out
}

fn trace(input) {
    let a = chars(input.text1);
    let b = chars(input.text2);
    let m = a.len();
    let n = b.len();

    let dp = [];
    for r in 0..(m + 1) {
        let row = [];
        for c in 0..(n + 1) { row.push(0); }
        dp.push(row);
    }

    enter("longestCommonSubsequence", #{ text1: S(input.text1), text2: S(input.text2) });
    step("init", `Compare "${input.text1}" with "${input.text2}".`,
        [ array("text1", a), array("text2", b) ]);
    step("alloc",
        "The table has one extra row and column for the empty prefix, which removes every bounds check.",
        [ grid("dp", as_strings(dp)).labels(labels_for(a), labels_for(b)) ]);

    for i in 1..(m + 1) {
        step("outer", `text1 up to '${a[i - 1]}'.`,
            [ grid("dp", as_strings(dp)).labels(labels_for(a), labels_for(b)).cur([i, 0]) ]);

        for j in 1..(n + 1) {
            step("inner", `Against text2 up to '${b[j - 1]}'.`,
                [ grid("dp", as_strings(dp)).labels(labels_for(a), labels_for(b)).cur([i, j]),
                  array("text1", a).hl([i - 1]), array("text2", b).hl([j - 1]) ]);

            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
                log(`match '${a[i - 1]}' -> ${dp[i][j]}`);
                step("match", `Both end in '${a[i - 1]}', so that letter can be appended to the best match of the two shorter prefixes.`,
                    [ grid("dp", as_strings(dp)).labels(labels_for(a), labels_for(b))
                        .hl([[i, j]]).done([[i - 1, j - 1]]),
                      array("text1", a).hl([i - 1]), array("text2", b).hl([j - 1]) ]);
                step("diag", `dp[${i}][${j}] = dp[${i - 1}][${j - 1}] + 1 = ${dp[i][j]}.`,
                    [ grid("dp", as_strings(dp)).labels(labels_for(a), labels_for(b)).hl([[i, j]]) ]);
            } else {
                let up = dp[i - 1][j];
                let left = dp[i][j - 1];
                step("cmp", `'${a[i - 1]}' and '${b[j - 1]}' differ, so one of them must be dropped.`,
                    [ grid("dp", as_strings(dp)).labels(labels_for(a), labels_for(b))
                        .cur([i, j]).done([[i - 1, j], [i, j - 1]]),
                      array("text1", a).bad([i - 1]), array("text2", b).bad([j - 1]) ]);

                if up >= left {
                    dp[i][j] = up;
                    step("up", `Dropping text1's letter gives ${up}, which is at least as good as ${left}.`,
                        [ grid("dp", as_strings(dp)).labels(labels_for(a), labels_for(b))
                            .hl([[i, j]]).done([[i - 1, j]]) ]);
                } else {
                    dp[i][j] = left;
                    step("left", `Dropping text2's letter gives ${left}, better than ${up}.`,
                        [ grid("dp", as_strings(dp)).labels(labels_for(a), labels_for(b))
                            .hl([[i, j]]).done([[i, j - 1]]) ]);
                }
            }
        }
    }

    let answer = dp[m][n];
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `The bottom-right cell covers both whole strings: ${answer}.`,
        [ grid("dp", as_strings(dp)).labels(labels_for(a), labels_for(b)).hl([[m, n]]) ],
        "return");
    leave(`${answer}`);
}
