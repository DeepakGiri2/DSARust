// Min Cost Climbing Stairs — solved backwards, because the future is what is
// known.
//
// dp[i] is "the total cost of a journey that starts by standing on step i".
// That depends on steps further up, so filling the table from the top down
// means every value it needs is already there.

fn trace(input) {
    let cost = input.cost;
    let n = cost.len();
    let dp = [];
    for i in 0..(n + 2) { dp.push(0); }

    enter("minCostClimbingStairs", #{ cost: A(cost) });
    set(#{ dp: A(dp) });
    step("init", "dp[i] = the cheapest way to finish, starting from step i. Past the top it costs nothing.",
        [ bars("cost", cost), array("dp", dp) ]);

    let i = n - 1;
    while i >= 0 {
        set(#{ i: N(i) });
        step("loop", `Step ${i} costs ${cost[i]}.`,
            [ bars("cost", cost).ptr("i", i).hl([i]).done(after(i, n)),
              array("dp", dp).ptr("i", i).done(after(i, n + 2)) ]);

        let one = dp[i + 1];
        step("one", `Climbing one step lands on ${i + 1}, which already costs ${one} to finish from.`,
            [ array("dp", dp).ptr("i", i).hl([i + 1]) ]);

        let two = 0;
        step("twoInit", "Climbing two is the other option.", [ array("dp", dp).ptr("i", i) ]);
        if i + 2 <= n {
            two = dp[i + 2];
            step("hasTwo", `Step ${i + 2} exists.`, [ array("dp", dp).ptr("i", i).hl([i + 2]) ]);
            step("twoSet", `It costs ${two} to finish from there.`,
                [ array("dp", dp).ptr("i", i).hl([i + 2]) ]);
        } else {
            step("hasTwo", "Two steps would go past the top, which is free.",
                [ array("dp", dp).ptr("i", i) ]);
            step("twoSet", "So that option costs 0.", [ array("dp", dp).ptr("i", i) ]);
        }

        let best = if one < two { one } else { two };
        dp[i] = cost[i] + best;
        set(#{ dp: A(dp) });
        log(`dp[${i}] = ${cost[i]} + ${best} = ${dp[i]}`);
        step("set", `dp[${i}] = ${cost[i]} + min(${one}, ${two}) = ${dp[i]}.`,
            [ bars("cost", cost).hl([i]), array("dp", dp).hl([i]) ]);
        i -= 1;
    }

    let answer = if dp[0] < dp[1] { dp[0] } else { dp[1] };
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `You may start on step 0 (${dp[0]}) or step 1 (${dp[1]}), so the answer is ${answer}.`,
        [ array("dp", dp).hl([0, 1]) ], "return");
    leave(`${answer}`);
}
