// Find Pivot Index — the right-hand sum is a subtraction, not a second loop.
//
// The animation is built around one identity: once the whole array has been
// added up, the sum to the right of index i is total − left − nums[i]. That is
// why a problem that reads like "compare two sums at every index" (O(n²)) is
// really one pass with one running variable.

const SUMS = "running sums";

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("pivotIndex", #{ nums: A(nums) });

    let total = 0;
    set(#{ total: N(total) });
    step("init", "Start with total = 0. The first pass only needs the grand total — nothing clever yet.",
        [ array("nums", nums), text(global::SUMS, ["total = 0"]) ]);

    for i in 0..n {
        total += nums[i];
        set(#{ total: N(total) });
        step("sum", `Add nums[${i}] = ${nums[i]} → total = ${total}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)).hl([i]),
              text(global::SUMS, [`total = ${total}`]) ]);
    }

    let left = 0;
    set(#{ left: N(left) });
    unset("i");
    step("left", `The whole array sums to ${total}. Now walk it again carrying left — the sum of everything strictly before the current index.`,
        [ array("nums", nums), text(global::SUMS, [`total = ${total}`, "left  = 0"]) ]);

    for i in 0..n {
        let x = nums[i];
        set(#{ i: N(i), x: N(x) });
        step("loop", `i = ${i}: is ${x} the pivot? left is the sum of nums[0..${i}] = ${left}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)).hl([i]),
              text(global::SUMS, [`total = ${total}`, `left  = ${left}`]) ]);

        let right = total - left - x;
        set(#{ right: N(right) });
        step("right",
            `right = total − left − nums[${i}] = ${total} − ${left} − ${x} = ${right}. No second loop: the right-hand sum is one subtraction.`,
            [ array("nums", nums).ptr("i", i).done(before(i)).hl([i]),
              text(global::SUMS, [`total = ${total}`, `left  = ${left}`, `right = ${right}`]) ]);

        if left == right {
            log(`pivot at ${i}: ${left} == ${right}`, "result");
            step("check", `${left} == ${right} — the two sides balance at index ${i}.`,
                [ array("nums", nums).ptr("i", i).done(before(i)).hl([i]),
                  text(global::SUMS, [`left  = ${left}`, `right = ${right}`, "balanced ✓"]) ]);
            result(`${i}`);
            step("ret", `Return ${i}. It is the leftmost such index because the scan runs left to right.`,
                [ array("nums", nums).hl([i]).done(before(i)),
                  text(global::SUMS, [`pivot index = ${i}`]) ],
                "return");
            leave(`${i}`);
            return;
        }

        step("check", `${left} ≠ ${right}, so index ${i} is not a pivot.`,
            [ array("nums", nums).ptr("i", i).done(before(i)).bad([i]),
              text(global::SUMS, [`left  = ${left}`, `right = ${right}`, "not balanced"]) ]);

        left += x;
        set(#{ left: N(left) });
        step("advance", `Move past ${x}: it now belongs to the left side, so left = ${left}.`,
            [ array("nums", nums).ptr("i", i).done(span(0, i)),
              text(global::SUMS, [`total = ${total}`, `left  = ${left}`]) ]);
    }

    log("no pivot index", "result");
    result("-1");
    step("none", "Every index was checked and none balanced — return -1.",
        [ array("nums", nums).done(seq(0, n - 1)), text(global::SUMS, ["no pivot"]) ],
        "return");
    leave("-1");
}
