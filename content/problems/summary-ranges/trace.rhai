// Summary Ranges — sorted and distinct means "consecutive" is a local test.
//
// Both guarantees are load-bearing. Sorted means a run can only extend to the
// right; distinct means nums[i+1] == nums[i] + 1 is exactly the condition for
// the run to continue. Drop either and this needs a different algorithm.

const A = "nums";

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("summaryRanges", #{ nums: A(nums) });

    let out = [];
    step("init", "Collect the ranges as they close.",
        [ array(global::A, nums), text("ranges", ["(none yet)"]) ]);

    let i = 0;
    while i < n {
        set(#{ i: N(i) });
        step("loop", `i = ${i}, value ${nums[i]}.`,
            [ array(global::A, nums).ptr("i", i).done(before(i)) ]);

        let start = i;
        set(#{ start: N(start) });
        step("start", `A new run begins at ${nums[start]}.`,
            [ array(global::A, nums).ptr("start", start).hl([start]).done(before(start)) ]);

        while i + 1 < n && nums[i + 1] == nums[i] + 1 {
            step("extend", `nums[${i + 1}] = ${nums[i + 1]} is exactly one more than ${nums[i]} — the run continues.`,
                [ array(global::A, nums).ptr("start", start).ptr("i", i + 1).hl(span(start, i + 1)) ]);
            i += 1;
            set(#{ i: N(i) });
            step("grow", `The run now covers ${nums[start]} … ${nums[i]}.`,
                [ array(global::A, nums).ptr("start", start).ptr("i", i).hl(span(start, i)) ]);
        }

        if start == i {
            step("single", `${nums[i]} stands alone — ${if i + 1 < n { `the next value is ${nums[i + 1]}, not ${nums[i] + 1}` } else { "there is no next value" }}.`,
                [ array(global::A, nums).hl([i]).done(before(start)) ]);
            out.push(`${nums[i]}`);
            step("one", `Emit "${nums[i]}" — a single number gets no arrow.`,
                [ text("ranges", out) ]);
        } else {
            step("single", `The run holds ${plural(i - start + 1, "value")}, so it needs the arrow form.`,
                [ array(global::A, nums).hl(span(start, i)).done(before(start)) ]);
            out.push(`${nums[start]}->${nums[i]}`);
            step("range", `Emit "${nums[start]}->${nums[i]}".`,
                [ array(global::A, nums).hl(span(start, i)), text("ranges", out) ]);
        }

        i += 1;
        step("next", `Move past the run to index ${i}.`,
            [ array(global::A, nums).done(before(i)) ]);
    }

    let answer = join(out, ",");
    log(answer, "result");
    result(answer);
    step("ret", `${plural(out.len(), "range")}: ${answer}.`,
        [ array(global::A, nums).done(seq(0, n - 1)), text("ranges", out) ], "return");
    leave(answer);
}
