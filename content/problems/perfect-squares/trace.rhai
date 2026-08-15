// Perfect Squares — greedy fails, so try every square and reuse the answers.
//
// The tempting move, take the biggest square that fits, gets 12 wrong: it
// gives 9 + 1 + 1 + 1 where 4 + 4 + 4 is better. So every square has to be
// tried at every total. What makes that affordable is that each smaller total
// has already been solved exactly once — the table below is never recomputed,
// only read.

fn trace(input) {
    let n = input.n;

    enter("numSquares", #{ n: N(n) });

    let dp = [0];
    for i in 1..(n + 1) { dp.push(0); }
    step("init", "dp[i] will hold the fewest squares summing to i. dp[0] is 0 — nothing needs no squares.",
        [ array("dp", dp) ]);

    for i in 1..(n + 1) {
        step("loop", `Solving for ${i}.`,
            [ array("dp", dp).ptr("i", i).done(before(i)) ]);

        dp[i] = i;
        step("worst", `Start from the worst case: ${i} copies of 1² always works, so ${i} is a safe upper bound.`,
            [ array("dp", dp).ptr("i", i).hl([i]) ]);

        let k = 1;
        while k * k <= i {
            step("inner", `Try ${k}² = ${k * k}, leaving ${i - k * k}.`,
                [ array("dp", dp).pointers(#{ rest: i - k * k, i: i }).hl([i - k * k]) ]);

            if dp[i - k * k] + 1 < dp[i] {
                step("better", `${i - k * k} needs ${dp[i - k * k]} square(s), so ${i} could be done in ${dp[i - k * k] + 1} — better than ${dp[i]}.`,
                    [ array("dp", dp).pointers(#{ rest: i - k * k, i: i }).hl([i - k * k]) ]);
                dp[i] = dp[i - k * k] + 1;
                log(`${i} = ${k * k} + (${i - k * k}) in ${dp[i]} square(s)`);
                step("update", `dp[${i}] = ${dp[i]}.`,
                    [ array("dp", dp).ptr("i", i).hl([i]) ]);
            } else {
                step("better", `${i - k * k} needs ${dp[i - k * k]}, so this route costs ${dp[i - k * k] + 1} — no better than the ${dp[i]} already found.`,
                    [ array("dp", dp).pointers(#{ rest: i - k * k, i: i }) ]);
            }
            k += 1;
        }
    }

    log(`answer ${dp[n]}`, "result");
    result(`${dp[n]}`);
    step("ret", `${n} needs ${plural(dp[n], "perfect square")}.`,
        [ bars("dp", dp).ptr("n", n).hl([n]) ], "return");
    leave(`${dp[n]}`);
}
