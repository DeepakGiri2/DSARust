// First Missing Positive — the array is the hash table.
//
// Two observations do all the work. First, with n slots the answer cannot
// exceed n+1, so every value outside 1…n is irrelevant and can be ignored
// where it lies. Second, value v belongs at index v−1, so the array can be
// permuted into place with swaps and no extra memory.
//
// The nested loop looks quadratic and is not: every swap puts one value at its
// final index, and a value never leaves once it is home, so there are at most
// n swaps in total.

const A = "nums";

fn homed(nums) {
    // Indices already holding their own value.
    let out = [];
    for i in 0..nums.len() {
        if nums[i] == i + 1 { out.push(i); }
    }
    out
}

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("firstMissingPositive", #{ nums: A(nums), n: N(n) });

    step("n", `${n} slots, so the answer is somewhere in 1 … ${n + 1}. Anything negative, zero, or bigger than ${n} cannot be the answer and is left where it is.`,
        [ array(global::A, nums) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `i = ${i}: nums[${i}] = ${nums[i]}.`,
            [ array(global::A, nums).ptr("i", i).done(homed(nums)) ]);

        while nums[i] > 0 && nums[i] <= n && nums[nums[i] - 1] != nums[i] {
            let v = nums[i];
            step("can", `${v} is in 1…${n} and index ${v - 1} does not already hold it — so ${v} is not home yet.`,
                [ array(global::A, nums).ptr("i", i).hl([i, v - 1]).done(homed(nums)) ]);

            let j = v - 1;
            set(#{ j: N(j) });
            step("home", `${v} belongs at index ${j}.`,
                [ array(global::A, nums).ptr("i", i).ptr("home", j).done(homed(nums)) ]);

            swap_at(nums, i, j);
            set(#{ nums: A(nums) });
            step("swap", `Swap. ${v} is now home for good, and whatever was at ${j} lands at ${i} to be dealt with next.`,
                [ array(global::A, nums).ptr("i", i).hl([i, j]).done(homed(nums)) ]);
        }

        step("can", `nums[${i}] = ${nums[i]} — either out of range, or already where it belongs. Move on.`,
            [ array(global::A, nums).ptr("i", i).done(homed(nums)) ]);
    }

    unset("j");
    for i in 0..n {
        set(#{ i: N(i) });
        step("check", `Index ${i} should hold ${i + 1}; it holds ${nums[i]}.`,
            [ array(global::A, nums).ptr("i", i).done(before(i)) ]);
        if nums[i] != i + 1 {
            log(`first gap at index ${i}`, "result");
            step("gap", `Mismatch — ${i + 1} is missing from the array.`,
                [ array(global::A, nums).ptr("i", i).bad([i]).done(before(i)) ]);
            result(`${i + 1}`);
            step("ret", `Return ${i + 1}.`,
                [ array(global::A, nums).bad([i]).done(before(i)) ], "return");
            leave(`${i + 1}`);
            return;
        }
    }

    log(`1 … ${n} all present`, "result");
    result(`${n + 1}`);
    step("all", `Every index holds its own value, so 1 … ${n} are all present and the answer is ${n + 1}.`,
        [ array(global::A, nums).done(seq(0, n - 1)) ], "return");
    leave(`${n + 1}`);
}
