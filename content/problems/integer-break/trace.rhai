// Integer Break — two options per split, and that is where "at least two
// parts" is enforced.
//
// dp[i] means "the best product from breaking i into two or more parts". The
// subtlety is that dp[i - j] answers a *different* question than "what is the
// remainder worth": it insists the remainder is broken up too. Sometimes that
// is worse. So both are tried — leave the remainder whole, or break it — and
// the larger wins.

fn trace(input) {
    let n = input.n;

    enter("integerBreak", #{ n: N(n) });

    let dp = [];
    for i in 0..(n + 1) { dp.push(0); }
    step("init", "dp[i] is the best product obtainable by breaking i into two or more parts.",
        [ array("dp", dp) ]);

    dp[1] = 1;
    step("seed", "dp[1] = 1 is used only as a factor — 1 cannot itself be broken, but leaving it whole is worth 1.",
        [ array("dp", dp).hl([1]) ]);

    for i in 2..(n + 1) {
        step("loop", `Solving for ${i}.`,
            [ array("dp", dp).ptr("i", i).done(before(i)) ]);

        for j in 1..i {
            step("split", `Make the first part ${j}, leaving ${i - j}.`,
                [ array("dp", dp).pointers(#{ j: j, rest: i - j, i: i }) ]);

            let best = j * (i - j);
            step("plain", `Leave the remainder whole: ${j} × ${i - j} = ${best}. This is the option that keeps the split to exactly two parts.`,
                [ text("options", [`${j} × ${i - j} = ${best}`]) ]);

            if j * dp[i - j] > best {
                step("deeper", `Or break the remainder too: ${j} × dp[${i - j}] = ${j} × ${dp[i - j]} = ${j * dp[i - j]}, which is more.`,
                    [ array("dp", dp).ptr("rest", i - j).hl([i - j]),
                      text("options", [`${j} × ${i - j} = ${best}`,
                                       `${j} × dp[${i - j}] = ${j * dp[i - j]}`]) ]);
                best = j * dp[i - j];
                step("useDp", `So this split is worth ${best}.`,
                    [ text("options", [`best for j = ${j}: ${best}`]) ]);
            } else {
                step("deeper", `Breaking the remainder gives ${j} × ${dp[i - j]} = ${j * dp[i - j]}, which is no better — leaving ${i - j} whole wins.`,
                    [ array("dp", dp).ptr("rest", i - j),
                      text("options", [`${j} × ${i - j} = ${best}`,
                                       `${j} × dp[${i - j}] = ${j * dp[i - j]}`]) ]);
            }

            if best > dp[i] {
                step("improve", `${best} beats the ${dp[i]} recorded so far for ${i}.`,
                    [ array("dp", dp).ptr("i", i) ]);
                dp[i] = best;
                log(`dp[${i}] = ${best} (first part ${j})`);
                step("store", `dp[${i}] = ${dp[i]}.`,
                    [ array("dp", dp).ptr("i", i).hl([i]) ]);
            } else {
                step("improve", `${best} does not beat ${dp[i]}, so nothing changes.`,
                    [ array("dp", dp).ptr("i", i) ]);
            }
        }
    }

    log(`answer ${dp[n]}`, "result");
    result(`${dp[n]}`);
    step("ret", `The best product from breaking ${n} up is ${dp[n]}.`,
        [ bars("dp", dp).ptr("n", n).hl([n]) ], "return");
    leave(`${dp[n]}`);
}
