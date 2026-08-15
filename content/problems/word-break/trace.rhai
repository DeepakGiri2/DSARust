// Word Break — dp[i] is "the first i characters can be segmented".
//
// The naive search re-explores the same suffixes over and over. Anchoring the
// subproblem at a *prefix length* means each position is decided once: a word
// can end here if the text before it was already reachable.

fn flags(dp) {
    let out = [];
    for v in dp { out.push(if v { "T" } else { "F" }); }
    out
}

fn trace(input) {
    let cs = chars(input.s);
    let n = cs.len();
    let dict = words(input.dict);
    let dp = [];
    for i in 0..(n + 1) { dp.push(false); }

    enter("wordBreak", #{ s: S(input.s) });
    step("init", "dp[i] answers: can the first i characters be split into dictionary words?",
        [ array("s", cs), text("dictionary", dict), array("dp", flags(dp)) ]);

    dp[0] = true;
    set(#{ dp: A(flags(dp)) });
    step("base", "dp[0] is true — the empty prefix is trivially segmentable.",
        [ array("dp", flags(dp)).hl([0]) ]);

    for i in 1..(n + 1) {
        set(#{ i: N(i) });
        step("outer", `Can the first ${i} character(s), "${join(cs.extract(0, i), "")}", be segmented?`,
            [ array("s", cs).window(0, i - 1), array("dp", flags(dp)).ptr("i", i).done(before(i)) ]);

        for w in dict {
            let m = w.len();
            step("inner", `Could "${w}" be the last word?`,
                [ array("s", cs).window(0, i - 1), text("dictionary", dict).hl([dict.index_of(w)]) ]);

            if i >= m && dp[i - m] && join(cs.extract(i - m, m), "") == w {
                step("check",
                    `"${w}" does end at ${i}, and dp[${i - m}] is already true — so everything before it splits too.`,
                    [ array("s", cs).hl(span(i - m, i - 1)),
                      array("dp", flags(dp)).hl([i - m]) ]);
                dp[i] = true;
                set(#{ dp: A(flags(dp)) });
                log(`dp[${i}] = true via "${w}"`);
                step("set", `dp[${i}] = true.`,
                    [ array("s", cs).hl(span(i - m, i - 1)), array("dp", flags(dp)).hl([i]) ]);
                step("brk", "One way is enough — stop trying words for this position.",
                    [ array("dp", flags(dp)).hl([i]) ]);
                break;
            }
            let why = if i < m {
                `"${w}" is longer than the prefix`
            } else if join(cs.extract(i - m, m), "") != w {
                `the text before ${i} is not "${w}"`
            } else {
                `the prefix before "${w}" cannot be segmented`
            };
            step("check", `No: ${why}.`,
                [ array("s", cs).window(0, i - 1),
                  array("dp", flags(dp)).ptr("i", i) ]);
        }
    }

    log(`answer ${dp[n]}`, "result");
    result(`${dp[n]}`);
    step("ret", `dp[${n}] covers the whole string: ${dp[n]}.`,
        [ array("s", cs), array("dp", flags(dp)).hl([n]) ], "return");
    leave(`${dp[n]}`);
}
