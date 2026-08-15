// Range Sum Query — the whole idea is the off-by-one that isn't one.
//
// prefix has n+1 slots, not n: prefix[0] = 0 means "sum of nothing". That
// single extra cell is what makes sum(left..right) = prefix[right+1] −
// prefix[left] work for every range including one starting at 0, with no
// if-statement anywhere.

fn parse_queries(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

fn labels_for(n) {
    let out = [];
    for i in 0..(n + 1) { out.push(`p${i}`); }
    out
}

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let queries = parse_queries(input.queries);

    enter("Constructor", #{ nums: A(nums) });

    let prefix = [];
    for i in 0..(n + 1) { prefix.push(0); }
    set(#{ prefix: A(prefix) });
    step("init",
        `prefix has ${n + 1} slots for ${n} values. prefix[0] = 0 is the sum of no elements at all — the cell that removes every special case later.`,
        [ array("nums", nums), array("prefix", prefix).hl([0]) ]);

    for i in 0..n {
        step("build", `i = ${i}: extend the prefix by one element.`,
            [ array("nums", nums).ptr("i", i).done(before(i)).hl([i]),
              array("prefix", prefix).done(span(0, i)) ]);
        prefix[i + 1] = prefix[i] + nums[i];
        set(#{ prefix: A(prefix) });
        step("add",
            `prefix[${i + 1}] = prefix[${i}] + nums[${i}] = ${prefix[i]} + ${nums[i]} = ${prefix[i + 1]} — the sum of the first ${i + 1} values.`,
            [ array("nums", nums).ptr("i", i).done(before(i)).hl([i]),
              array("prefix", prefix).done(span(0, i)).hl([i + 1]) ]);
    }

    step("ready", `Built. Every range sum from now on costs one subtraction, no matter how long the range is.`,
        [ array("nums", nums).done(seq(0, n - 1)), array("prefix", prefix) ],
        "return");
    leave("built");

    let answers = [];
    for q in 0..queries.len() {
        let l = queries[q][0];
        let r = queries[q][1];

        enter(`sumRange(${l}, ${r})`, #{ left: N(l), right: N(r) });
        step("query", `Query ${q}: sum of nums[${l}..${r}] inclusive.`,
            [ array("nums", nums).window(l, r).hl(span(l, r)),
              array("prefix", prefix) ]);

        let hi = prefix[r + 1];
        let lo = prefix[l];
        let ans = hi - lo;
        set(#{ hi: N(hi), lo: N(lo) });
        log(`sumRange(${l}, ${r}) = ${hi} - ${lo} = ${ans}`, "result");
        step("answer",
            `prefix[${r + 1}] − prefix[${l}] = ${hi} − ${lo} = ${ans}. The first term is everything up to ${r}; the second peels off everything before ${l}.`,
            [ array("nums", nums).window(l, r).hl(span(l, r)),
              array("prefix", prefix).hl([l, r + 1]) ],
            "return");
        leave(`${ans}`);
        answers.push(ans);
    }

    result(join(answers, " "));
}
