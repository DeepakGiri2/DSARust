// Next Permutation — find the pivot, swap minimally, then reset the tail.
//
// Read the array from the right: as long as it keeps increasing (going left to
// right it is decreasing), that suffix is already the biggest arrangement of
// its values and nothing inside it can grow. The first place that breaks the
// pattern is the only digit that can be raised, and it must be raised by as
// little as possible — hence the smallest value to its right that still beats
// it. After the swap the tail is still non-increasing, so reversing it gives
// the smallest possible ending.

const A = "nums";

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("nextPermutation", #{ nums: A(nums) });

    let i = n - 2;
    set(#{ i: N(i) });
    step("start", `Start one from the right, at index ${i}. The last element alone can never be raised.`,
        [ array(global::A, nums).ptr("i", i) ]);

    while i >= 0 && nums[i] >= nums[i + 1] {
        step("descend", `nums[${i}] = ${nums[i]} ≥ nums[${i + 1}] = ${nums[i + 1]} — the suffix from ${i} is still non-increasing, so it is already at its maximum.`,
            [ array(global::A, nums).ptr("i", i).done(after(i, n)) ]);
        i -= 1;
        set(#{ i: N(i) });
        step("back", `Move left to ${i}.`,
            [ array(global::A, nums).ptr("i", i).done(after(i, n)) ]);
    }

    if i >= 0 {
        step("pivot", `nums[${i}] = ${nums[i]} < nums[${i + 1}] = ${nums[i + 1]} — index ${i} is the pivot. Everything to its right is the largest it can be, so the pivot is the digit that has to grow.`,
            [ array(global::A, nums).ptr("pivot", i).hl([i]).done(after(i, n)) ]);

        let j = n - 1;
        set(#{ j: N(j) });
        step("scan", `Now find the smallest value to the right that still beats ${nums[i]}. The suffix is non-increasing, so scanning from the far right finds it first.`,
            [ array(global::A, nums).ptr("pivot", i).ptr("j", j).hl([i]) ]);

        while nums[j] <= nums[i] {
            step("bigger", `nums[${j}] = ${nums[j]} ≤ ${nums[i]} — too small to be an increase.`,
                [ array(global::A, nums).ptr("pivot", i).ptr("j", j).bad([j]) ]);
            j -= 1;
            set(#{ j: N(j) });
            step("downj", `Try index ${j}.`,
                [ array(global::A, nums).ptr("pivot", i).ptr("j", j) ]);
        }
        step("bigger", `nums[${j}] = ${nums[j]} > ${nums[i]}, and it is the smallest such value — anything further left in the suffix is at least as large.`,
            [ array(global::A, nums).ptr("pivot", i).ptr("j", j).hl([i, j]) ]);

        swap_at(nums, i, j);
        set(#{ nums: A(nums) });
        step("swap", `Swap them: the permutation just got bigger, by the least it possibly could.`,
            [ array(global::A, nums).ptr("pivot", i).hl([i, j]) ]);
    } else {
        step("pivot", "No pivot exists — the whole array is non-increasing, so this is the *largest* permutation. The problem says wrap around to the smallest.",
            [ array(global::A, nums).done(seq(0, n - 1)) ]);
    }

    step("flip", `Reverse everything after index ${i}. It is non-increasing, so reversing makes it the smallest arrangement of those values — the smallest possible tail.`,
        [ array(global::A, nums).window(i + 1, n - 1) ]);

    let lo = i + 1;
    let hi = n - 1;
    while lo < hi {
        swap_at(nums, lo, hi);
        set(#{ nums: A(nums) });
        step("mirror", `Swap positions ${lo} and ${hi}.`,
            [ array(global::A, nums).ptr("lo", lo).ptr("hi", hi).hl([lo, hi]).window(i + 1, n - 1) ]);
        lo += 1;
        hi -= 1;
    }

    let out = join(nums, " ");
    log(out, "result");
    result(out);
    step("flip", `Next permutation: ${brackets(nums)}.`,
        [ array(global::A, nums).done(seq(0, n - 1)) ], "return");
    leave(brackets(nums));
}
