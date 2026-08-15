// Delete and Earn — the hard part is the change of variable.
//
// Stated over the array, the rule is awkward: taking one element deletes
// others somewhere else entirely. Stated over the *values*, it is familiar —
// two copies of 3 never interfere, so bucket the points by value and the rule
// becomes "no two adjacent values". That is House Robber, and the remainder of
// the code is the standard two-variable version of it.

fn labels(points) {
    let out = [];
    for v in 0..points.len() { out.push(v); }
    out
}

fn trace(input) {
    let nums = input.nums;

    enter("deleteAndEarn", #{});

    let max_v = 0;
    for v in nums { if v > max_v { max_v = v; } }
    set(#{ maxV: N(max_v) });
    step("scan", `The largest value present is ${max_v}, so the value line runs 1..${max_v}.`,
        [ array("nums", nums) ]);

    let points = [];
    for v in 0..(max_v + 1) { points.push(0); }
    step("bucket", "One bucket per value: how many points taking that value is worth in total.",
        [ array("nums", nums), bars("points at each value", points) ]);

    for k in 0..nums.len() {
        let v = nums[k];
        step("fill", `Bucketing ${v}.`,
            [ array("nums", nums).ptr("k", k).done(before(k)),
              bars("points at each value", points) ]);
        points[v] = points[v] + v;
        step("add", `Every copy of ${v} adds another ${v}: bucket ${v} now holds ${points[v]}. Copies never conflict with each other, which is exactly why they can be merged.`,
            [ array("nums", nums).ptr("k", k),
              bars("points at each value", points).hl([v]) ]);
    }

    let prev2 = 0;
    let prev1 = 0;
    let dp = [0];
    set(#{ prev2: N(prev2), prev1: N(prev1) });
    step("init", "Now it is House Robber along the value line: taking value v forbids v-1 and v+1.",
        [ bars("points at each value", points),
          text("state", [`best up to v-2 = ${prev2}`, `best up to v-1 = ${prev1}`]) ]);

    for v in 1..(max_v + 1) {
        step("loop", `Deciding about value ${v}, worth ${points[v]}.`,
            [ bars("points at each value", points).ptr("v", v).hl([v]),
              text("state", [`best up to ${v - 2} = ${prev2}`, `best up to ${v - 1} = ${prev1}`]) ]);

        let take = prev2 + points[v];
        step("take", `Take it: ${points[v]} points plus the best from ${v - 2} back, since ${v - 1} would be wiped out. ${points[v]} + ${prev2} = ${take}.`,
            [ bars("points at each value", points).hl([v]),
              text("choice", [`take = ${take}`]) ]);

        let skip = prev1;
        step("skip", `Or skip it and keep the best up to ${v - 1}: ${skip}.`,
            [ bars("points at each value", points).ptr("v", v),
              text("choice", [`take = ${take}`, `skip = ${skip}`]) ]);

        let best = if take > skip { take } else { skip };
        log(`best up to ${v} is ${best}`);
        step("best", `Best through value ${v} is ${best}.`,
            [ text("choice", [`take = ${take}`, `skip = ${skip}`, `best = ${best}`]) ]);

        prev2 = prev1;
        prev1 = best;
        dp.push(best);
        set(#{ prev2: N(prev2), prev1: N(prev1) });
        step("shift", "Slide the window forward.",
            [ bars("best total through each value", dp).hl([dp.len() - 1]) ]);
    }

    log(`answer ${prev1}`, "result");
    result(`${prev1}`);
    step("ret", `The most that can be earned is ${prev1}.`,
        [ bars("best total through each value", dp).hl([dp.len() - 1]) ], "return");
    leave(`${prev1}`);
}
