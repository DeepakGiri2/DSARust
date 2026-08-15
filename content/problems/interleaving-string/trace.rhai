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

// Interleaving String — two indices are enough because they fix the third.
//
// Greedy fails: when both strings offer the same character, taking the wrong
// one can doom the rest. So every reachable state is kept. The economy comes
// from noticing that consuming i characters of s1 and j of s2 must have
// consumed exactly i+j of s3 — the third position is not free, so the table is
// two-dimensional rather than three.

fn trace(input) {
    let s1 = input.s1;
    let s2 = input.s2;
    let s3 = input.s3;
    let a = chars(s1);
    let b = chars(s2);
    let c = chars(s3);
    let m = a.len();
    let n = b.len();

    enter("isInterleave", #{ m: N(m), n: N(n) });

    if m + n != c.len() {
        log("lengths do not add up", "result");
        step("length", `${m} + ${n} = ${m + n}, but s3 is ${c.len()} long — no interleaving can change the total length.`,
            [ text("lengths", [`s1: ${m}`, `s2: ${n}`, `s3: ${c.len()}`]) ], "return");
        result("false");
        leave("false");
        return;
    }
    step("length", `${m} + ${n} = ${c.len()}, so the lengths at least allow it.`,
        [ text("lengths", [`s1: ${m}`, `s2: ${n}`, `s3: ${c.len()}`]) ]);

    let dp = [];
    for i in 0..(m + 1) {
        let row = [];
        for j in 0..(n + 1) { row.push(false); }
        dp.push(row);
    }
    step("init", "Cell (i, j) asks: can the first i of s1 and the first j of s2 build the first i+j of s3?",
        [ grid("dp — rows are s1, columns are s2", bool_table(dp))
            .labels(headers(s1), headers(s2)) ]);

    grid_set(dp, 0, 0, true);
    step("seed", "Two empty prefixes build an empty prefix, trivially.",
        [ grid("dp — rows are s1, columns are s2", bool_table(dp))
            .labels(headers(s1), headers(s2)).hl([[0, 0]]) ]);

    for i in 0..(m + 1) {
        step("rowLoop", `Using ${plural(i, "character")} of s1.`,
            [ grid("dp — rows are s1, columns are s2", bool_table(dp))
                .labels(headers(s1), headers(s2)).cur([i, 0]) ]);

        for j in 0..(n + 1) {
            if i == 0 && j == 0 {
                step("origin", "The origin is already settled.",
                    [ grid("dp — rows are s1, columns are s2", bool_table(dp))
                        .labels(headers(s1), headers(s2)).cur([0, 0]) ]);
                continue;
            }

            let want = c[i + j - 1];
            step("colLoop", `Cell (${i},${j}): the next character of s3 to place is "${want}", at position ${i + j - 1}. Notice that i and j alone decided that.`,
                [ array("s3", c).ptr("i+j-1", i + j - 1).hl([i + j - 1]).done(before(i + j - 1)),
                  grid("dp — rows are s1, columns are s2", bool_table(dp))
                      .labels(headers(s1), headers(s2)).cur([i, j]) ]);

            let ok = false;
            step("ok", "Assume unreachable until one of the two predecessors proves otherwise.",
                [ grid("dp — rows are s1, columns are s2", bool_table(dp))
                    .labels(headers(s1), headers(s2)).cur([i, j]) ]);

            if i > 0 && a[i - 1] == want && dp[i - 1][j] {
                step("fromS1", `s1's ${i}th character is "${a[i - 1]}", which matches, and (${i - 1},${j}) is reachable.`,
                    [ grid("dp — rows are s1, columns are s2", bool_table(dp))
                        .labels(headers(s1), headers(s2)).cur([i, j]).hl([[i - 1, j]]) ]);
                ok = true;
                step("takeS1", `So "${want}" can come from s1.`,
                    [ grid("dp — rows are s1, columns are s2", bool_table(dp))
                        .labels(headers(s1), headers(s2)).cur([i, j]) ]);
            } else {
                step("fromS1",
                    if i == 0 { "s1 has contributed nothing yet, so it cannot have supplied this character.".to_string() }
                    else if a[i - 1] != want { `s1 offers "${a[i - 1]}", not "${want}".` }
                    else { `s1 offers "${want}", but (${i - 1},${j}) is not reachable, so that route does not exist.` },
                    [ grid("dp — rows are s1, columns are s2", bool_table(dp))
                        .labels(headers(s1), headers(s2)).cur([i, j]) ]);
            }

            if j > 0 && b[j - 1] == want && dp[i][j - 1] {
                step("fromS2", `s2's ${j}th character is "${b[j - 1]}", which matches, and (${i},${j - 1}) is reachable.`,
                    [ grid("dp — rows are s1, columns are s2", bool_table(dp))
                        .labels(headers(s1), headers(s2)).cur([i, j]).hl([[i, j - 1]]) ]);
                ok = true;
                step("takeS2", `So "${want}" can come from s2. Both routes being possible is exactly the case a greedy walk gets wrong.`,
                    [ grid("dp — rows are s1, columns are s2", bool_table(dp))
                        .labels(headers(s1), headers(s2)).cur([i, j]) ]);
            } else {
                step("fromS2",
                    if j == 0 { "s2 has contributed nothing yet.".to_string() }
                    else if b[j - 1] != want { `s2 offers "${b[j - 1]}", not "${want}".` }
                    else { `s2 offers "${want}", but (${i},${j - 1}) is not reachable.` },
                    [ grid("dp — rows are s1, columns are s2", bool_table(dp))
                        .labels(headers(s1), headers(s2)).cur([i, j]) ]);
            }

            grid_set(dp, i, j, ok);
            step("store", `(${i},${j}) is ${if ok { "reachable" } else { "unreachable" }}.`,
                [ grid("dp — rows are s1, columns are s2", bool_table(dp))
                    .labels(headers(s1), headers(s2)).cur([i, j]) ]);
        }
    }

    let answer = dp[m][n];
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `Consuming all of s1 and all of s2 ${if answer { "does" } else { "does not" }} produce s3.`,
        [ grid("dp — rows are s1, columns are s2", bool_table(dp))
            .labels(headers(s1), headers(s2)).hl([[m, n]]) ], "return");
    leave(`${answer}`);
}
