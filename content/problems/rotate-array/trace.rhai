// Rotate Array — three reversals, no second array.
//
// Reversing everything already puts the right elements in the right *halves*;
// it just leaves each half backwards. Reversing the halves undoes that. The
// juggling-algorithm and extra-array solutions are both valid answers; this
// one is the one interviewers are usually fishing for, because it is O(1)
// space and three lines.

const A = "nums";

fn rev(nums, lo, hi, tag) {
    enter(`reverse(${lo}, ${hi})`, #{ lo: N(lo), hi: N(hi) });
    let l = lo;
    let h = hi;
    while l < h {
        step("loop", `lo = ${l}, hi = ${h}.`,
            [ array(global::A, nums).ptr("lo", l).ptr("hi", h).window(lo, hi) ]);
        swap_at(nums, l, h);
        step("swap", `Swap ${nums[h]} and ${nums[l]}.`,
            [ array(global::A, nums).ptr("lo", l).ptr("hi", h).hl([l, h]).window(lo, hi) ]);
        l += 1;
        h -= 1;
        step("move", "Step both pointers inward.",
            [ array(global::A, nums).ptr("lo", l).ptr("hi", h).window(lo, hi) ]);
    }
    step("loop", `Pointers met — the block ${lo} … ${hi} is reversed.`,
        [ array(global::A, nums).window(lo, hi).done(span(lo, hi)) ], "return");
    leave(brackets(nums));
    nums
}

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let k = input.k;

    enter("rotate", #{ nums: A(nums), k: N(k) });

    let kk = k % n;
    set(#{ k: N(kk) });
    step("norm", `k = ${k} mod ${n} = ${kk}. Rotating by more than the length just wraps around, so this one line handles k = 0 and k = 1000 alike.`,
        [ array(global::A, nums) ]);

    step("rev1", "Reverse the whole array. The last k elements are now at the front — backwards, but in the right half.",
        [ array(global::A, nums) ], "call");
    nums = rev(nums, 0, n - 1, "rev1");

    step("rev2", `Reverse the first ${kk} of them to put that block back in order.`,
        [ array(global::A, nums).window(0, kk - 1) ], "call");
    nums = rev(nums, 0, kk - 1, "rev2");

    step("rev3", `Reverse the remaining ${n - kk}.`,
        [ array(global::A, nums).window(kk, n - 1) ], "call");
    nums = rev(nums, kk, n - 1, "rev3");

    let out = join(nums, " ");
    log(out, "result");
    result(out);
    step("norm", `Rotated right by ${kk}: ${brackets(nums)}. Every element was written exactly twice and nothing was allocated.`,
        [ array(global::A, nums).done(seq(0, n - 1)) ], "return");
    leave(brackets(nums));
}
