// Target Sum — merge the branches that have become indistinguishable.
//
// A brute force explores 2^n sign assignments. But once two assignments reach
// the same running total, nothing about the remaining numbers can tell them
// apart — they will succeed or fail together. Keeping a count per total rather
// than a list of assignments collapses all of them, and the number of distinct
// totals is bounded by the sum, not by 2^n.

fn trace(input) {
    let nums = input.nums;
    let target = input.target;

    enter("findTargetSumWays", #{ target: N(target) });

    let ways = map_new();
    map_set(ways, 0, 1);
    step("init", "Before any number is signed, the running total is 0, reached exactly one way.",
        [ array("nums", nums), kv("running total → ways", ways) ]);

    for i in 0..nums.len() {
        let v = nums[i];
        step("loop", `Choosing the sign of nums[${i}] = ${v}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              kv("running total → ways", ways) ]);

        let next = map_new();
        step("fresh", "Build the next tally separately — writing into the current one would let a number be signed twice.",
            [ kv("running total → ways", ways), kv("next tally", next) ]);

        for entry in ways {
            let s = entry[0];
            let c = entry[1];
            step("scan", `${plural(c, "way")} of reaching ${s}.`,
                [ kv("running total → ways", ways).hl([`${s}`]), kv("next tally", next) ]);

            map_inc(next, s + v, c);
            step("plus", `Choosing + sends all ${c} of them to ${s + v}.`,
                [ kv("next tally", next).hl([`${s + v}`]) ]);

            map_inc(next, s - v, c);
            step("minus", `Choosing − sends the same ${c} to ${s - v}. Any assignment already counted at ${s - v} is now merged with these, because nothing later can distinguish them.`,
                [ kv("next tally", next).hl([`${s - v}`]) ]);
        }

        ways = next;
        log(`after ${i + 1} number(s): ${map_len(ways)} distinct total(s)`);
        step("swap", `${plural(map_len(ways), "distinct total")} after ${i + 1} of ${nums.len()} numbers — far fewer than the ${pow2(i + 1)} sign assignments they stand for.`,
            [ kv("running total → ways", ways) ]);
    }

    let answer = if map_has(ways, target) { map_get(ways, target) } else { 0 };
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `${plural(answer, "assignment")} evaluate to ${target}.`,
        [ kv("running total → ways", ways).hl([`${target}`]) ], "return");
    leave(`${answer}`);
}

fn pow2(k) {
    let v = 1;
    for i in 0..k { v *= 2; }
    v
}
