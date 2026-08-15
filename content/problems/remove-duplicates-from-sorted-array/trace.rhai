// Remove Duplicates from Sorted Array — a slow write pointer.
//
// Sorted input means every copy of a value sits together, so a value is new
// exactly when it differs from the last one kept. `slow` marks the end of the
// answer being built in place; `fast` does the reading.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("removeDuplicates", #{ nums: A(nums) });

    if n == 0 {
        step("empty", "The array is empty.", [ array("nums", nums) ]);
        result("0");
        step("ret0", "Nothing to keep — return 0.", [ array("nums", nums) ], "return");
        leave("0");
        return;
    }
    step("empty", `${n} values, already sorted.`, [ array("nums", nums) ]);

    let slow = 0;
    set(#{ slow: N(slow) });
    step("init", "slow marks the last kept value. nums[0] is always kept.",
        [ array("nums", nums).ptr("slow", slow).hl([0]) ]);

    for fast in 1..n {
        set(#{ fast: N(fast) });
        step("loop", `fast = ${fast}: is nums[${fast}] = ${nums[fast]} new?`,
            [ array("nums", nums).pointers(#{ slow: slow, fast: fast })
                .hl(seq(0, slow)).done(before(fast)) ]);

        if nums[fast] != nums[slow] {
            step("cmp",
                `${nums[fast]} differs from the last kept value ${nums[slow]}. The array is sorted, so this is a value we have never kept.`,
                [ array("nums", nums).pointers(#{ slow: slow, fast: fast })
                    .hl(seq(0, slow)).done(before(fast)) ]);
            slow += 1;
            set(#{ slow: N(slow) });
            step("incS", `Make room: slow = ${slow}.`,
                [ array("nums", nums).pointers(#{ slow: slow, fast: fast }).done(before(fast)) ]);
            nums[slow] = nums[fast];
            log(`keep ${nums[slow]} at ${slow}`);
            step("write", `Write ${nums[slow]} into position ${slow}.`,
                [ array("nums", nums).pointers(#{ slow: slow, fast: fast })
                    .hl(seq(0, slow)).done(before(fast)) ]);
        } else {
            step("cmp", `${nums[fast]} is the same as the last kept value — a duplicate, so skip it.`,
                [ array("nums", nums).pointers(#{ slow: slow, fast: fast })
                    .bad([fast]).hl(seq(0, slow)) ]);
        }
    }

    let k = slow + 1;
    log(`${k} unique value(s)`, "result");
    result(`${k}`);
    step("ret", `The first ${k} positions hold every distinct value, in order.`,
        [ array("nums", nums).hl(seq(0, slow)).done(after(slow, n)) ], "return");
    leave(`${k}`);
}
