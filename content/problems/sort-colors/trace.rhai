// Sort Colors — the Dutch national flag partition.
//
// Three regions grow at once: everything below `low` is 0, everything above
// `high` is 2, and `mid` walks the unknown middle. The asymmetry to notice is
// that swapping with `high` does *not* advance mid — the value that just
// arrived has not been looked at yet.

fn regions(low, mid, high, n) {
    #{ low: low, mid: mid, high: high }
}

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let low = 0;
    let mid = 0;
    let high = n - 1;

    enter("sortColors", #{ nums: A(nums) });
    set(#{ low: N(low), mid: N(mid), high: N(high) });
    step("init",
        "Three pointers: below low is settled 0s, above high is settled 2s, and mid scans the unknown middle.",
        [ array("nums", nums).pointers(regions(low, mid, high, n)).window(mid, high) ]);

    while mid <= high {
        step("loop", `The unknown region is [${mid}..${high}].`,
            [ array("nums", nums).pointers(regions(low, mid, high, n)).window(mid, high)
                .done(dead(n, mid, high)) ]);

        if nums[mid] == 0 {
            step("is0", `nums[${mid}] = 0, so it belongs at the front.`,
                [ array("nums", nums).pointers(regions(low, mid, high, n)).hl([mid]) ]);
            swap_at(nums, low, mid);
            log(`swap ${low} and ${mid}`);
            step("swaplo", `Swap it into position ${low}.`,
                [ array("nums", nums).pointers(regions(low, mid, high, n)).hl([low, mid]) ]);
            low += 1;
            set(#{ low: N(low) });
            step("inclow", `low = ${low}.`,
                [ array("nums", nums).pointers(regions(low, mid, high, n)).window(mid, high) ]);
            mid += 1;
            set(#{ mid: N(mid) });
            step("incmid", `mid = ${mid}. The value swapped out of low was already scanned, so mid may advance.`,
                [ array("nums", nums).pointers(regions(low, mid, high, n)).window(mid, high)
                    .done(dead(n, mid, high)) ]);
        } else if nums[mid] == 1 {
            step("is1", `nums[${mid}] = 1 — the middle colour is already where it belongs.`,
                [ array("nums", nums).pointers(regions(low, mid, high, n)).hl([mid]) ]);
            mid += 1;
            set(#{ mid: N(mid) });
            step("justmid", `mid = ${mid}.`,
                [ array("nums", nums).pointers(regions(low, mid, high, n)).window(mid, high)
                    .done(dead(n, mid, high)) ]);
        } else {
            step("swaphi", `nums[${mid}] = 2, so it belongs at the back — swap it with position ${high}.`,
                [ array("nums", nums).pointers(regions(low, mid, high, n)).hl([mid, high]) ]);
            swap_at(nums, mid, high);
            log(`swap ${mid} and ${high}`);
            high -= 1;
            set(#{ high: N(high) });
            step("dechigh",
                `high = ${high}. mid stays put: the value that just came back from the end has never been examined.`,
                [ array("nums", nums).pointers(regions(low, mid, high, n)).window(mid, high)
                    .done(dead(n, mid, high)) ]);
        }
    }

    let out = brackets(nums);
    log(`sorted ${out}`, "result");
    result(out);
    step("loop", `mid passed high, so the unknown region is empty: ${out}. One pass, no counting.`,
        [ array("nums", nums).hl(seq(0, n - 1)) ], "return");
    leave(out);
}
