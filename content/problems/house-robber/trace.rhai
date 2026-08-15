// House Robber — a two-cell rolling DP.
//
// dp[i] answers "best haul from the first i+1 houses", and each answer only
// ever looks two cells back, which is why the table can collapse to two
// numbers. The table view keeps both so the recurrence stays visible.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let dp = [];
    for i in 0..n { dp.push(0); }

    enter("rob", #{ nums: A(nums) });
    set(#{ n: N(n) });
    step("init", `${n} houses. dp[i] will hold the best haul from houses 0..i.`,
        [ array("houses", nums), array("dp", dp) ]);

    step("dp", "The table starts empty.", [ array("houses", nums), array("dp", dp) ]);

    dp[0] = nums[0];
    set(#{ dp: A(dp) });
    step("base", `With one house there is no choice: dp[0] = ${nums[0]}.`,
        [ array("houses", nums).hl([0]), array("dp", dp).hl([0]) ]);

    for i in 1..n {
        set(#{ i: N(i) });
        step("loop", `House ${i} holds ${nums[i]}. Rob it, or skip it?`,
            [ array("houses", nums).ptr("i", i).done(before(i)), array("dp", dp).done(before(i)) ]);

        let skip = dp[i - 1];
        step("skip", `Skipping it keeps the best from houses 0..${i - 1}: ${skip}.`,
            [ array("houses", nums).ptr("i", i).done(before(i)),
              array("dp", dp).hl([i - 1]).done(before(i - 1)) ]);

        let two_back = if i >= 2 { dp[i - 2] } else { 0 };
        step("robval", `Robbing it takes ${nums[i]} plus the best from houses 0..${i - 2} (${two_back}) — house ${i - 1} must be left alone.`,
            [ array("houses", nums).ptr("i", i).hl([i]).bad([i - 1]).done(before(i - 1)),
              array("dp", dp).hl(if i >= 2 { [i - 2] } else { [] }).done(before(i)) ]);

        step("guard", "Two houses back does not exist for i = 1, in which case it counts as 0.",
            [ array("houses", nums).ptr("i", i).done(before(i)), array("dp", dp).done(before(i)) ]);

        let rob = nums[i] + two_back;
        step("add", `Robbing gives ${nums[i]} + ${two_back} = ${rob}.`,
            [ array("houses", nums).ptr("i", i).hl([i]).done(before(i)),
              array("dp", dp).done(before(i)) ]);

        if rob > skip {
            step("cmp", `${rob} beats ${skip}, so robbing house ${i} is better.`,
                [ array("houses", nums).ptr("i", i).hl([i]).done(before(i)),
                  array("dp", dp).done(before(i)) ]);
            dp[i] = rob;
            set(#{ dp: A(dp) });
            log(`dp[${i}] = ${rob} (rob)`);
            step("takerob", `dp[${i}] = ${rob}.`,
                [ array("houses", nums).ptr("i", i).hl([i]).done(before(i)),
                  array("dp", dp).hl([i]).done(before(i)) ]);
        } else {
            step("cmp", `${skip} is at least as good as ${rob}, so skipping wins.`,
                [ array("houses", nums).ptr("i", i).done(before(i)),
                  array("dp", dp).done(before(i)) ]);
            dp[i] = skip;
            set(#{ dp: A(dp) });
            log(`dp[${i}] = ${skip} (skip)`);
            step("takeskip", `dp[${i}] = ${skip}.`,
                [ array("houses", nums).ptr("i", i).done(before(i)),
                  array("dp", dp).hl([i]).done(before(i)) ]);
        }
    }

    let answer = dp[n - 1];
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `dp[${n - 1}] = ${answer} is the most that can be taken without robbing two adjacent houses.`,
        [ array("houses", nums), array("dp", dp).hl([n - 1]) ], "return");
    leave(`${answer}`);
}
