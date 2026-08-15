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

// Regular Expression Matching — '*' belongs to the character before it.
//
// The whole difficulty is that '*' is not a symbol to be matched; it modifies
// the element in front of it, so "a*" has to be handled as one unit two
// pattern-cells wide. Once that is accepted the table is short: zero uses of
// the group means jumping two columns left, and one more use means climbing
// one row while staying in the same column, because the pattern has not
// advanced. Row 0 is not decoration either — it is where "a*b*" gets to match
// the empty string.

fn validate(input) {
    let cs = chars(input.p);
    if cs.len() > 0 && cs[0] == "*" {
        return "the pattern cannot begin with '*' — there is nothing for it to repeat";
    }
    for i in 1..cs.len() {
        if cs[i] == "*" && cs[i - 1] == "*" {
            return "'*' cannot follow another '*'";
        }
    }
}

fn trace(input) {
    let s = input.s;
    let p = input.p;
    let a = chars(s);
    let b = chars(p);
    let m = a.len();
    let n = b.len();

    enter("isMatch", #{ m: N(m), n: N(n) });

    let dp = [];
    for i in 0..(m + 1) {
        let row = [];
        for j in 0..(n + 1) { row.push(false); }
        dp.push(row);
    }
    step("init", "dp[i][j]: does the first i characters of s match the first j of p?",
        [ grid("dp — rows are s, columns are p", bool_table(dp)).labels(headers(s), headers(p)) ]);

    grid_set(dp, 0, 0, true);
    step("seed", "An empty pattern matches an empty string.",
        [ grid("dp — rows are s, columns are p", bool_table(dp)).labels(headers(s), headers(p))
            .hl([[0, 0]]) ]);

    for j in 1..(n + 1) {
        step("emptyRow", `Row 0, column ${j}: can "${prefix_of(b, j)}" match the empty string?`,
            [ grid("dp — rows are s, columns are p", bool_table(dp)).labels(headers(s), headers(p))
                .cur([0, j]) ]);
        if b[j - 1] == "*" {
            step("starEmpty", `"${b[j - 2]}*" can be used zero times, so it can vanish.`,
                [ grid("dp — rows are s, columns are p", bool_table(dp)).labels(headers(s), headers(p))
                    .cur([0, j]).hl([[0, j - 2]]) ]);
            grid_set(dp, 0, j, dp[0][j - 2]);
            step("copyEmpty", `So this cell is whatever it was two columns back: ${dp[0][j]}.`,
                [ grid("dp — rows are s, columns are p", bool_table(dp)).labels(headers(s), headers(p))
                    .cur([0, j]) ]);
        } else {
            step("starEmpty", `"${b[j - 1]}" must consume a character, and there are none — so no.`,
                [ grid("dp — rows are s, columns are p", bool_table(dp)).labels(headers(s), headers(p))
                    .cur([0, j]) ]);
        }
    }

    for i in 1..(m + 1) {
        step("rowLoop", `Now s[${i - 1}] = "${a[i - 1]}".`,
            [ grid("dp — rows are s, columns are p", bool_table(dp)).labels(headers(s), headers(p))
                .cur([i, 0]) ]);

        for j in 1..(n + 1) {
            step("colLoop", `Against p[${j - 1}] = "${b[j - 1]}".`,
                [ grid("dp — rows are s, columns are p", bool_table(dp)).labels(headers(s), headers(p))
                    .cur([i, j]) ]);

            if b[j - 1] == "*" {
                step("isStar", `A '*', so the unit really being matched is "${b[j - 2]}*".`,
                    [ grid("dp — rows are s, columns are p", bool_table(dp))
                        .labels(headers(s), headers(p)).cur([i, j]) ]);

                grid_set(dp, i, j, dp[i][j - 2]);
                step("zeroUse", `Use it zero times: skip both pattern characters and take the answer from two columns left, ${dp[i][j]}.`,
                    [ grid("dp — rows are s, columns are p", bool_table(dp))
                        .labels(headers(s), headers(p)).cur([i, j]).hl([[i, j - 2]]) ]);

                if b[j - 2] == "." || b[j - 2] == a[i - 1] {
                    step("canRepeat", `"${b[j - 2]}" also matches "${a[i - 1]}", so the group may be used once more.`,
                        [ grid("dp — rows are s, columns are p", bool_table(dp))
                            .labels(headers(s), headers(p)).cur([i, j]).hl([[i - 1, j]]) ]);
                    grid_set(dp, i, j, dp[i][j] || dp[i - 1][j]);
                    step("moreUse", `That consumes one character of s but leaves the pattern in place — so it reads from directly above: ${dp[i][j]}.`,
                        [ grid("dp — rows are s, columns are p", bool_table(dp))
                            .labels(headers(s), headers(p)).cur([i, j]) ]);
                } else {
                    step("canRepeat", `"${b[j - 2]}" does not match "${a[i - 1]}", so the group cannot absorb this character.`,
                        [ grid("dp — rows are s, columns are p", bool_table(dp))
                            .labels(headers(s), headers(p)).cur([i, j]) ]);
                }
            } else if b[j - 1] == "." || b[j - 1] == a[i - 1] {
                step("single", `"${b[j - 1]}" matches "${a[i - 1]}" — one character each, no repetition involved.`,
                    [ grid("dp — rows are s, columns are p", bool_table(dp))
                        .labels(headers(s), headers(p)).cur([i, j]).hl([[i - 1, j - 1]]) ]);
                grid_set(dp, i, j, dp[i - 1][j - 1]);
                step("shift", `Both advance, so the answer comes from up-left: ${dp[i][j]}.`,
                    [ grid("dp — rows are s, columns are p", bool_table(dp))
                        .labels(headers(s), headers(p)).cur([i, j]) ]);
            } else {
                step("single", `"${b[j - 1]}" cannot match "${a[i - 1]}", and there is no '*' to fall back on.`,
                    [ grid("dp — rows are s, columns are p", bool_table(dp))
                        .labels(headers(s), headers(p)).cur([i, j]) ]);
            }
        }
    }

    let answer = dp[m][n];
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `"${p}" ${if answer { "matches" } else { "does not match" }} "${s}".`,
        [ grid("dp — rows are s, columns are p", bool_table(dp)).labels(headers(s), headers(p))
            .hl([[m, n]]) ], "return");
    leave(`${answer}`);
}

fn prefix_of(cs, k) {
    let out = "";
    for i in 0..k { out += cs[i]; }
    out
}
