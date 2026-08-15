// Majority Element — Boyer-Moore voting.
//
// Think of it as every element cancelling one of a different kind. A value
// appearing more than n/2 times cannot be fully cancelled by all the others
// put together, so whoever is left standing is the majority.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let count = 0;
    let candidate = 0;

    enter("majorityElement", #{ nums: A(nums) });
    set(#{ count: N(count) });
    step("init", "count is how far ahead the current candidate is.", [ array("nums", nums) ]);
    set(#{ candidate: NIL() });
    step("cand", "No candidate yet.", [ array("nums", nums) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `i = ${i}: nums[${i}] = ${nums[i]}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              text("state", [`candidate = ${if count == 0 && i == 0 { "none" } else { `${candidate}` }}`,
                            `count = ${count}`]) ]);

        if count == 0 {
            step("zero", "The lead is zero — every vote so far has been cancelled out.",
                [ array("nums", nums).ptr("i", i).done(before(i)) ]);
            candidate = nums[i];
            set(#{ candidate: N(candidate) });
            log(`new candidate ${candidate}`);
            step("pick", `Adopt ${candidate} as the new candidate.`,
                [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)),
                  text("state", [`candidate = ${candidate}`, `count = ${count}`]) ]);
        }

        if nums[i] == candidate {
            step("same", `${nums[i]} agrees with the candidate.`,
                [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)) ]);
            count += 1;
            set(#{ count: N(count) });
            step("up", `count = ${count}.`,
                [ array("nums", nums).ptr("i", i).done(before(i)),
                  text("state", [`candidate = ${candidate}`, `count = ${count}`]) ]);
        } else {
            step("same", `${nums[i]} disagrees, so it cancels one vote for ${candidate}.`,
                [ array("nums", nums).ptr("i", i).bad([i]).done(before(i)) ]);
            count -= 1;
            set(#{ count: N(count) });
            step("down", `count = ${count}.`,
                [ array("nums", nums).ptr("i", i).done(before(i)),
                  text("state", [`candidate = ${candidate}`, `count = ${count}`]) ]);
        }
    }

    log(`majority ${candidate}`, "result");
    result(`${candidate}`);
    step("ret",
        `${candidate} survived every cancellation. A value appearing more than ${n}/2 times has more votes than everything else combined, so it cannot be cancelled away.`,
        [ array("nums", nums).hl(seq(0, n - 1)) ], "return");
    leave(`${candidate}`);
}
