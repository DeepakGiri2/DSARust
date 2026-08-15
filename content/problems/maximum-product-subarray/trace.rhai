// Maximum Product Subarray — the minimum is not junk.
//
// With sums, only the running maximum matters. With products, a negative
// number flips the sign of everything, so the most negative running product
// becomes the most positive one. Carrying both and swapping them on a negative
// is what keeps a single pass correct.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let best = nums[0];
    let cur_max = nums[0];
    let cur_min = nums[0];

    enter("maxProduct", #{ nums: A(nums) });
    set(#{ best: N(best), curMax: N(cur_max), curMin: N(cur_min) });
    step("init", "Track both the largest and the smallest product ending here.",
        [ array("nums", nums).ptr("i", 0).hl([0]),
          text("running", [`max = ${cur_max}`, `min = ${cur_min}`, `best = ${best}`]) ]);

    for i in 1..n {
        let v = nums[i];
        set(#{ i: N(i) });
        step("loop", `i = ${i}, value ${v}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              text("running", [`max = ${cur_max}`, `min = ${cur_min}`, `best = ${best}`]) ]);

        if v < 0 {
            step("negative", `${v} is negative, so multiplying flips which of the two is larger.`,
                [ array("nums", nums).ptr("i", i).bad([i]),
                  text("running", [`max = ${cur_max}`, `min = ${cur_min}`]) ]);
            let t = cur_max;
            cur_max = cur_min;
            cur_min = t;
            set(#{ curMax: N(cur_max), curMin: N(cur_min) });
            step("swap", `Swap them: max = ${cur_max}, min = ${cur_min}.`,
                [ text("running", [`max = ${cur_max}`, `min = ${cur_min}`]) ]);
        } else {
            step("negative", `${v} is not negative, so the roles stay put.`,
                [ array("nums", nums).ptr("i", i) ]);
        }

        let prod_max = cur_max * v;
        cur_max = if v > prod_max { v } else { prod_max };
        set(#{ curMax: N(cur_max) });
        step("setMax", `Either extend (${prod_max}) or start fresh at ${v}: max = ${cur_max}.`,
            [ array("nums", nums).ptr("i", i),
              text("running", [`max = ${cur_max}`, `min = ${cur_min}`]) ]);

        let prod_min = cur_min * v;
        cur_min = if v < prod_min { v } else { prod_min };
        set(#{ curMin: N(cur_min) });
        step("setMin", `Same for the minimum: ${cur_min}. It is kept because a later negative could make it the winner.`,
            [ text("running", [`max = ${cur_max}`, `min = ${cur_min}`]) ]);

        if cur_max > best {
            step("cmp", `${cur_max} beats the best product so far (${best}).`,
                [ array("nums", nums).ptr("i", i).hl([i]) ]);
            best = cur_max;
            set(#{ best: N(best) });
            log(`new best ${best}`, "result");
            step("setBest", `Best = ${best}.`,
                [ text("running", [`max = ${cur_max}`, `min = ${cur_min}`, `best = ${best}`]) ]);
        } else {
            step("cmp", `${cur_max} does not beat ${best}.`,
                [ array("nums", nums).ptr("i", i) ]);
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `The largest product of any contiguous run is ${best}.`,
        [ array("nums", nums).done(seq(0, n - 1)) ], "return");
    leave(`${best}`);
}
