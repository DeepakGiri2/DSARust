// Subarray Sum Equals K — a subarray sum is a difference of prefix sums.
//
// The sum of nums[i..j] is prefix(j) − prefix(i−1). So asking for a subarray
// ending at j that sums to k is asking whether prefix(i−1) = prefix(j) − k has
// ever occurred — a lookup, not a scan. Counts rather than a set, because the
// same running total can be reached several times and each occurrence is a
// distinct subarray.
//
// Seeding the map with 0 → 1 is not a special case bolted on: the empty prefix
// really does have sum 0, and it is what a subarray starting at index 0
// subtracts.

fn trace(input) {
    let nums = input.nums;
    let k = input.k;
    let n = nums.len();

    enter("subarraySum", #{ k: N(k) });

    let seen = map_new();
    map_set(seen, 0, 1);
    step("init", "How many times each running total has occurred. The empty prefix sums to 0, so 0 starts with a count of 1.",
        [ array("nums", nums), kv("running total → times seen", seen) ]);

    let sum = 0;
    let count = 0;
    set(#{ sum: N(sum), count: N(count) });
    step("running", "The running total, and the answer so far.",
        [ text("state", [`running total = ${sum}`, `subarrays found = ${count}`]) ]);

    for i in 0..n {
        step("loop", `Element ${i}: ${nums[i]}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              kv("running total → times seen", seen) ]);

        sum += nums[i];
        set(#{ sum: N(sum) });
        step("add", `Running total through index ${i} is ${sum}.`,
            [ array("nums", nums).ptr("i", i).hl(span(0, i)),
              text("state", [`running total = ${sum}`]) ]);

        let want = sum - k;
        if map_has(seen, want) {
            let c = map_get(seen, want);
            step("lookup", `A subarray ending here sums to ${k} exactly when some earlier prefix summed to ${sum} − ${k} = ${want}. That has happened ${plural(c, "time")}.`,
                [ kv("running total → times seen", seen).hl([`${want}`]),
                  text("state", [`running total = ${sum}`, `looking for = ${want}`]) ]);
            count += c;
            set(#{ count: N(count) });
            log(`+${c} subarray(s) ending at ${i}`);
            step("bump", `So there are ${c} more qualifying subarray(s): ${count} in total.`,
                [ text("state", [`subarrays found = ${count}`]) ]);
        } else {
            step("lookup", `No earlier prefix summed to ${sum} − ${k} = ${want}, so nothing ending here qualifies.`,
                [ kv("running total → times seen", seen),
                  text("state", [`running total = ${sum}`, `looking for = ${want}`]) ]);
            step("bump", `Count unchanged at ${count}.`,
                [ text("state", [`subarrays found = ${count}`]) ]);
        }

        map_inc(seen, sum, 1);
        step("record", `Record that ${sum} has now occurred ${plural(map_get(seen, sum), "time")}.`,
            [ kv("running total → times seen", seen).hl([`${sum}`]) ]);
    }

    log(`answer ${count}`, "result");
    result(`${count}`);
    step("ret", `${plural(count, "subarray")} sum to ${k}.`,
        [ array("nums", nums), text("state", [`subarrays found = ${count}`]) ], "return");
    leave(`${count}`);
}
