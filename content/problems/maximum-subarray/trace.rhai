// Maximum Subarray — Kadane's, as one decision repeated.
//
// At every index the only question is: does the running sum still help, or is
// this element better off starting fresh? The bars make the answer obvious the
// moment `cur` goes negative.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    let cur = nums[0];
    let best = nums[0];
    let start = 0;
    let best_lo = 0;
    let best_hi = 0;

    enter("maxSubArray", #{ nums: A(nums) });
    set(#{ cur: N(cur), best: N(best) });
    step("init", `With only nums[0] = ${nums[0]} seen, both the running sum and the best answer are ${nums[0]}.`,
        [ bars("nums", nums).ptr("i", 0).hl([0]) ]);

    for i in 1..n {
        set(#{ i: N(i) });
        step("loop", `i = ${i}: nums[${i}] = ${nums[i]}, running sum is ${cur}.`,
            [ bars("nums", nums).ptr("i", i).window(start, i - 1).done(before(start)) ]);

        if nums[i] > cur + nums[i] {
            step("cmp",
                `${nums[i]} on its own beats ${cur} + ${nums[i]} = ${cur + nums[i]} — the running sum has turned into a liability.`,
                [ bars("nums", nums).ptr("i", i).bad(span(start, i - 1)) ]);
            cur = nums[i];
            start = i;
            set(#{ cur: N(cur) });
            log(`restart at ${i}, cur = ${cur}`);
            step("reset", `Throw the old prefix away and start a new subarray at index ${i}.`,
                [ bars("nums", nums).ptr("i", i).window(start, i).done(before(start)) ]);
        } else {
            step("cmp", `${cur} + ${nums[i]} = ${cur + nums[i]} beats ${nums[i]} alone, so the prefix still pays.`,
                [ bars("nums", nums).ptr("i", i).window(start, i).done(before(start)) ]);
            cur = cur + nums[i];
            set(#{ cur: N(cur) });
            log(`extend, cur = ${cur}`);
            step("extend", `Extend the current subarray: running sum is ${cur}.`,
                [ bars("nums", nums).ptr("i", i).window(start, i).done(before(start)) ]);
        }

        if cur > best {
            step("cmpBest", `${cur} beats the best subarray sum so far (${best}).`,
                [ bars("nums", nums).ptr("i", i).window(start, i).done(before(start)) ]);
            best = cur;
            best_lo = start;
            best_hi = i;
            set(#{ best: N(best) });
            log(`new best ${best} over [${best_lo}..${best_hi}]`, "result");
            step("newBest", `Record ${best}, spanning [${best_lo}..${best_hi}].`,
                [ bars("nums", nums).ptr("i", i).hl(span(best_lo, best_hi)).done(before(start)) ]);
        } else {
            step("cmpBest", `${cur} does not beat ${best}.`,
                [ bars("nums", nums).ptr("i", i).window(start, i).hl(span(best_lo, best_hi)) ]);
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `The largest sum is ${best}, from the subarray [${best_lo}..${best_hi}].`,
        [ bars("nums", nums).hl(span(best_lo, best_hi)) ], "return");
    leave(`${best}`);
}
