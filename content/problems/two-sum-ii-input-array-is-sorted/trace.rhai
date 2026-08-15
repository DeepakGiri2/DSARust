// Two Sum II — the sorted order lets two pointers replace the hash map.
//
// The step to watch: when the sum is too small, only moving the *left* pointer
// can help, because everything left of r is already smaller than numbers[r].
// Each move eliminates a whole row of pairs at once.

fn trace(input) {
    let nums = input.numbers;
    let target = input.target;
    let n = nums.len();
    let label = `numbers   (sorted, target = ${target})`;

    let l = 0;
    let r = n - 1;

    enter("twoSumII", #{ numbers: A(nums), target: N(target) });
    set(#{ l: N(l), r: N(r) });
    step("init", "Start as wide as possible: the smallest value and the largest.",
        [ array(label, nums).pointers(#{ l: l, r: r }).window(l, r) ]);

    while l < r {
        step("loop", `The window [${l}..${r}] still holds at least two values.`,
            [ array(label, nums).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);

        let sum = nums[l] + nums[r];
        set(#{ sum: N(sum) });
        log(`${nums[l]} + ${nums[r]} = ${sum}`);
        step("sum", `numbers[${l}] + numbers[${r}] = ${nums[l]} + ${nums[r]} = ${sum}.`,
            [ array(label, nums).pointers(#{ l: l, r: r }).window(l, r)
                .hl([l, r]).done(dead(n, l, r)) ]);

        if sum == target {
            log(`answer: [${l + 1}, ${r + 1}]`, "result");
            step("cmp", `${sum} is exactly the target.`,
                [ array(label, nums).pointers(#{ l: l, r: r }).hl([l, r]) ]);
            result(`[${l + 1}, ${r + 1}]`);
            step("found", `Return 1-based indices [${l + 1}, ${r + 1}].`,
                [ array(label, nums).hl([l, r]) ], "return");
            leave(`[${l + 1}, ${r + 1}]`);
            return;
        }

        if sum < target {
            step("less",
                `${sum} < ${target}. numbers[${r}] is the largest value left, so no partner for numbers[${l}] exists inside the window — it can be discarded.`,
                [ array(label, nums).pointers(#{ l: l, r: r }).window(l, r)
                    .bad([l]).done(dead(n, l, r)) ]);
            l += 1;
            set(#{ l: N(l) });
            log(`sum too small -> l = ${l}`);
            step("moveL", `Move l to ${l} to get a bigger sum.`,
                [ array(label, nums).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);
        } else {
            step("less",
                `${sum} > ${target}. numbers[${l}] is the smallest value left, so numbers[${r}] is too big for any of them.`,
                [ array(label, nums).pointers(#{ l: l, r: r }).window(l, r)
                    .bad([r]).done(dead(n, l, r)) ]);
            r -= 1;
            set(#{ r: N(r) });
            log(`sum too big -> r = ${r}`);
            step("moveR", `Move r to ${r} to get a smaller sum.`,
                [ array(label, nums).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);
        }
    }

    log("pointers met with no pair", "result");
    result("[]");
    step("none", "The pointers met without ever hitting the target — no such pair exists.",
        [ array(label, nums).done(seq(0, n - 1)) ], "return");
    leave("[]");
}
