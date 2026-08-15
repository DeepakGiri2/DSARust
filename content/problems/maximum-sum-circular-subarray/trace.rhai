// Maximum Sum Circular Subarray — the wrap is a hole in the middle.
//
// There are only two shapes of answer. Either the best subarray does not wrap,
// which is plain Kadane, or it does — and then what it *excludes* is one
// contiguous block in the middle. Making that block as small as possible is
// Kadane again, minimising instead of maximising. Both run in the same pass.
//
// The trap is at the end: with every element negative, the minimum subarray is
// the entire array, so total − minSum is 0 — the empty subarray, which the
// problem forbids. maxSum > 0 is the guard.

const A = "nums";

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("maxSubarraySumCircular", #{ nums: A(nums) });

    let total = 0;
    let max_sum = nums[0];
    let cur_max = 0;
    let min_sum = nums[0];
    let cur_min = 0;
    set(#{ maxSum: N(max_sum), minSum: N(min_sum) });
    step("init", "Two Kadane runs share one pass: one looking for the best block, one for the worst.",
        [ array(global::A, nums),
          text("running", ["curMax = 0", "curMin = 0", `maxSum = ${max_sum}`, `minSum = ${min_sum}`]) ]);

    for i in 0..n {
        let x = nums[i];
        set(#{ i: N(i), x: N(x) });
        step("loop", `i = ${i}, value ${x}.`,
            [ array(global::A, nums).ptr("i", i).done(before(i)).hl([i]) ]);

        let extended = cur_max + x;
        cur_max = if extended > x { extended } else { x };
        set(#{ curMax: N(cur_max) });
        step("grow", `Best block ending here: either extend (${extended}) or start fresh at ${x} → ${cur_max}.`,
            [ array(global::A, nums).ptr("i", i).hl([i]),
              text("running", [`curMax = ${cur_max}`, `maxSum = ${max_sum}`]) ]);

        if cur_max > max_sum { max_sum = cur_max; }
        set(#{ maxSum: N(max_sum) });
        step("best", `maxSum = ${max_sum}.`,
            [ text("running", [`curMax = ${cur_max}`, `maxSum = ${max_sum}`]) ]);

        let shrunk = cur_min + x;
        cur_min = if shrunk < x { shrunk } else { x };
        set(#{ curMin: N(cur_min) });
        step("shrink", `Worst block ending here: ${cur_min}. This is the piece a wrapping answer would leave out.`,
            [ array(global::A, nums).ptr("i", i).bad([i]),
              text("running", [`curMin = ${cur_min}`, `minSum = ${min_sum}`]) ]);

        if cur_min < min_sum { min_sum = cur_min; }
        set(#{ minSum: N(min_sum) });
        step("worst", `minSum = ${min_sum}.`,
            [ text("running", [`curMin = ${cur_min}`, `minSum = ${min_sum}`]) ]);

        total += x;
        set(#{ total: N(total) });
        step("total", `total = ${total}.`,
            [ array(global::A, nums).done(span(0, i)),
              text("running", [`total = ${total}`, `maxSum = ${max_sum}`, `minSum = ${min_sum}`]) ]);
    }

    step("check", `maxSum = ${max_sum}. ${if max_sum > 0 { "It is positive, so at least one element is positive and the wrapped candidate is a real subarray." } else { "It is not positive, so every element is negative." }}`,
        [ text("summary", [`total  = ${total}`, `maxSum = ${max_sum}`, `minSum = ${min_sum}`,
                           `total − minSum = ${total - min_sum}`]) ]);

    let answer = max_sum;
    if max_sum > 0 {
        let wrapped = total - min_sum;
        if wrapped > answer { answer = wrapped; }
        log(`no wrap ${max_sum}, wrapped ${wrapped}`, "result");
        result(`${answer}`);
        step("wrap", `Wrapped candidate = total − minSum = ${total} − ${min_sum} = ${wrapped}. The answer is the better of ${max_sum} and ${wrapped}: ${answer}.`,
            [ array(global::A, nums),
              text("summary", [`no wrap  = ${max_sum}`, `wrapped  = ${wrapped}`, `answer   = ${answer}`]) ],
            "return");
    } else {
        log(`all negative, answer ${max_sum}`, "result");
        result(`${answer}`);
        step("allneg", `Everything is negative, so total − minSum would describe the empty subarray. Return the plain Kadane answer, ${max_sum}.`,
            [ array(global::A, nums), text("summary", [`answer = ${answer}`]) ],
            "return");
    }
    leave(`${answer}`);
}
