// Top K Frequent Elements — bucket by frequency, no sorting.
//
// A frequency can never exceed n, so it can be used directly as an array
// index. Walking the buckets from the top down yields values in decreasing
// frequency order in O(n) — no comparison sort, no heap.

fn bucket_rows(buckets) {
    let out = [];
    for f in 0..buckets.len() {
        out.push([`freq ${f}`, join(buckets[f], " ")]);
    }
    out
}

fn trace(input) {
    let nums = input.nums;
    let k = input.k;
    let n = nums.len();
    let count = map_new();

    enter("topKFrequent", #{ k: N(k) });
    set(#{ count: MAP(count) });
    step("count", "First count how often each value appears.",
        [ array("nums", nums), kv("count — value → times", count) ]);

    for i in 0..n {
        step("build", `Take nums[${i}] = ${nums[i]}.`,
            [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)),
              kv("count — value → times", count) ]);
        let c = map_inc(count, nums[i], 1);
        set(#{ count: MAP(count, [nums[i]]) });
        step("inc", `count[${nums[i]}] = ${c}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              kv("count — value → times", count).hl([nums[i]]) ]);
    }

    let buckets = [];
    for f in 0..(n + 1) { buckets.push([]); }
    step("buckets",
        `One bucket per possible frequency, 0 to ${n}. A value seen f times goes in bucket f — the frequency *is* the index.`,
        [ kv("count — value → times", count), kv("buckets", bucket_rows(buckets)) ]);

    let added = set_new();
    step("addedmap", "Each distinct value is filed once.", [ kv("count — value → times", count) ]);

    for i in 0..n {
        let v = nums[i];
        step("fill", `Consider ${v}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              kv("buckets", bucket_rows(buckets)) ]);

        if !set_has(added, v) {
            let f = map_get(count, v);
            step("isnew", `${v} has not been filed yet; it appears ${f} time(s).`,
                [ kv("count — value → times", count).hl([v]),
                  kv("buckets", bucket_rows(buckets)) ]);
            buckets[f].push(v);
            step("place", `Put ${v} in bucket ${f}.`,
                [ kv("buckets", bucket_rows(buckets)).hl([`freq ${f}`]) ]);
            set_add(added, v);
            step("mark", "Mark it filed.", [ kv("buckets", bucket_rows(buckets)) ]);
        } else {
            step("isnew", `${v} is already filed.`,
                [ array("nums", nums).ptr("i", i).bad([i]),
                  kv("buckets", bucket_rows(buckets)) ]);
        }
    }

    let res = [];
    set(#{ res: A(res) });
    step("res", "Now read the buckets from the highest frequency down.",
        [ kv("buckets", bucket_rows(buckets)) ]);

    let f = n;
    while f >= 1 {
        step("scan", `Bucket ${f}${if buckets[f].len() == 0 { " is empty" } else { `` }}.`,
            [ kv("buckets", bucket_rows(buckets)).hl([`freq ${f}`]),
              array("result", res) ]);

        for v in buckets[f] {
            step("take", `${v} appears ${f} time(s).`,
                [ kv("buckets", bucket_rows(buckets)).hl([`freq ${f}`]),
                  array("result", res) ]);
            if res.len() < k {
                step("room", `Still room — ${res.len()} of ${k} collected.`,
                    [ array("result", res) ]);
                res.push(v);
                set(#{ res: A(res) });
                log(`take ${v} (frequency ${f})`, "result");
                step("add", `Take ${v}.`, [ array("result", res).hl([res.len() - 1]) ]);
            }
        }
        f -= 1;
    }

    let answer = brackets(res);
    result(answer);
    step("return", `The ${k} most frequent values: ${answer} — found without ever sorting.`,
        [ array("result", res).hl(seq(0, res.len() - 1)) ], "return");
    leave(answer);
}
