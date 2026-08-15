// Subarray Sums Divisible by K — same remainder, divisible difference.
//
// sum(i..j) = prefix[j] − prefix[i-1]. That difference is a multiple of k
// precisely when the two prefixes leave the same remainder mod k, so the whole
// problem collapses to "how many earlier prefixes had this remainder?".
// The bucket for remainder 0 starts at 1 because the empty prefix is a real
// prefix — without it, no subarray starting at index 0 would ever be counted.

fn floor_mod(v, k) {
    let r = v % k;
    if r < 0 { r += k; }
    r
}

fn buckets(count, k) {
    let m = map_new();
    for r in 0..k { map_set(m, r, count[r]); }
    m
}

fn trace(input) {
    let nums = input.nums;
    let k = input.k;
    let n = nums.len();

    enter("subarraysDivByK", #{ nums: A(nums), k: N(k) });

    let count = [];
    for r in 0..k { count.push(0); }
    count[0] = 1;
    let sum = 0;
    let ans = 0;
    set(#{ sum: N(0), ans: N(0) });
    step("init", `One bucket per remainder: 0 … ${k - 1}. Every prefix sum will be filed under its remainder mod ${k}.`,
        [ array("nums", nums), kv("how many prefixes had this remainder", buckets(count, k)) ]);
    step("seed",
        "Bucket 0 starts at 1, standing for the empty prefix (sum 0). That is what lets a subarray that starts at index 0 be counted at all.",
        [ kv("how many prefixes had this remainder", buckets(count, k)).hl([0]) ]);
    step("zero", "running sum = 0, answer = 0.",
        [ array("nums", nums), text("state", ["sum = 0", "answer = 0"]) ]);

    for i in 0..n {
        let x = nums[i];
        set(#{ i: N(i), x: N(x) });
        step("loop", `i = ${i}: take nums[${i}] = ${x} into the running sum.`,
            [ array("nums", nums).ptr("i", i).done(before(i)).hl([i]),
              kv("how many prefixes had this remainder", buckets(count, k)) ]);

        sum += x;
        set(#{ sum: N(sum) });
        step("add", `sum = ${sum} — the total of nums[0..${i}].`,
            [ array("nums", nums).ptr("i", i).done(span(0, i)),
              text("state", [`sum = ${sum}`, `answer = ${ans}`]) ]);

        let r = floor_mod(sum, k);
        set(#{ r: N(r) });
        step("mod",
            `${sum} mod ${k} = ${r}. (The +k, %k dance is only there because ${sum} may be negative — Go, C++ and Java all give a negative remainder for a negative number.)`,
            [ array("nums", nums).ptr("i", i).done(span(0, i)),
              kv("how many prefixes had this remainder", buckets(count, k)).hl([r]) ]);

        let hits = count[r];
        ans += hits;
        set(#{ ans: N(ans) });
        if hits > 0 {
            log(`remainder ${r} seen ${hits}× before → +${hits}`, "result");
        }
        step("take",
            `${hits} earlier prefix(es) also left remainder ${r}; each pairs with this one to make a subarray divisible by ${k}. answer = ${ans}.`,
            [ array("nums", nums).ptr("i", i).done(span(0, i)),
              kv("how many prefixes had this remainder", buckets(count, k)).hl([r]),
              text("state", [`sum = ${sum}`, `remainder = ${r}`, `answer = ${ans}`]) ]);

        count[r] += 1;
        step("store", `File this prefix under remainder ${r} for later.`,
            [ kv("how many prefixes had this remainder", buckets(count, k)).hl([r]),
              text("state", [`sum = ${sum}`, `answer = ${ans}`]) ]);
    }

    log(`${ans} subarrays`, "result");
    result(`${ans}`);
    step("ret", `${ans} subarrays have a sum divisible by ${k}.`,
        [ array("nums", nums).done(seq(0, n - 1)),
          kv("how many prefixes had this remainder", buckets(count, k)),
          text("state", [`answer = ${ans}`]) ],
        "return");
    leave(`${ans}`);
}
