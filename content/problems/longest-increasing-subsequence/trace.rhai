// Longest Increasing Subsequence — dp[i] is the best run *ending* at i.
//
// Anchoring the subproblem at "ending here" is what makes the recurrence
// work: every earlier index with a smaller value is a legal predecessor, so
// dp[i] is one more than the best of them.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let dp = [];
    for i in 0..n { dp.push(1); }
    let best = 1;
    let best_at = 0;

    enter("lengthOfLIS", #{ nums: A(nums) });
    step("init", "dp[i] will hold the length of the longest increasing run that ends at index i.",
        [ array("nums", nums), array("dp", dp) ]);
    set(#{ dp: A(dp) });
    step("dp", "Every element is a run of length 1 on its own.",
        [ array("nums", nums), array("dp", dp) ]);
    set(#{ best: N(best) });
    step("best0", "Best so far: 1.", [ array("nums", nums), array("dp", dp) ]);

    for i in 1..n {
        set(#{ i: N(i) });
        step("outer", `Work out dp[${i}] for nums[${i}] = ${nums[i]}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              array("dp", dp).ptr("i", i).done(before(i)) ]);
        step("seed", `It is at least 1 — the element by itself.`,
            [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)),
              array("dp", dp).hl([i]).done(before(i)) ]);

        for j in 0..i {
            step("inner", `Could nums[${j}] = ${nums[j]} come before it?`,
                [ array("nums", nums).pointers(#{ j: j, i: i }).hl([j, i]).done(before(j)),
                  array("dp", dp).pointers(#{ j: j, i: i }) ]);

            if nums[j] < nums[i] && dp[j] + 1 > dp[i] {
                step("check", `${nums[j]} < ${nums[i]} and the run ending at ${j} is ${dp[j]} long, so extending it gives ${dp[j] + 1} — better than the ${dp[i]} recorded.`,
                    [ array("nums", nums).pointers(#{ j: j, i: i }).hl([j, i]),
                      array("dp", dp).hl([j]) ]);
                dp[i] = dp[j] + 1;
                set(#{ dp: A(dp) });
                log(`dp[${i}] = ${dp[i]}`);
                step("extend", `dp[${i}] = ${dp[i]}.`,
                    [ array("nums", nums).pointers(#{ j: j, i: i }).hl([j, i]),
                      array("dp", dp).hl([i]) ]);
            } else if nums[j] >= nums[i] {
                step("check", `${nums[j]} is not smaller than ${nums[i]}, so it cannot precede it in an increasing run.`,
                    [ array("nums", nums).pointers(#{ j: j, i: i }).bad([j]),
                      array("dp", dp) ]);
            } else {
                step("check", `Extending index ${j} would give ${dp[j] + 1}, no better than ${dp[i]}.`,
                    [ array("nums", nums).pointers(#{ j: j, i: i }),
                      array("dp", dp).hl([j]) ]);
            }
        }

        if dp[i] > best {
            step("cmp", `dp[${i}] = ${dp[i]} beats the best so far (${best}).`,
                [ array("nums", nums).hl([i]), array("dp", dp).hl([i]) ]);
            best = dp[i];
            best_at = i;
            set(#{ best: N(best) });
            log(`new best ${best} ending at ${i}`, "result");
            step("newbest", `Best is now ${best}.`,
                [ array("nums", nums).hl([i]), array("dp", dp).hl([i]) ]);
        } else {
            step("cmp", `dp[${i}] = ${dp[i]} does not beat ${best}.`,
                [ array("nums", nums).ptr("i", i), array("dp", dp).hl([best_at]) ]);
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `The longest increasing subsequence is ${best} long — the largest value in dp.`,
        [ array("nums", nums), array("dp", dp).hl([best_at]) ], "return");
    leave(`${best}`);
}
