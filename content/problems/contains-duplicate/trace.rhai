// Contains Duplicate — a set turns "have I seen this?" into one lookup.

fn trace(input) {
    let nums = input.nums;
    let seen = set_new();

    enter("containsDuplicate", #{ nums: A(nums) });
    set(#{ seen: SET(set_items(seen)) });
    step("init", "An empty set. Everything we walk past goes in, so the question becomes a single membership test.",
        [ array("nums", nums), kv("seen", seen) ]);

    for i in 0..nums.len() {
        let x = nums[i];
        set(#{ i: N(i), x: N(x) });
        step("loop", `i = ${i}: nums[${i}] = ${x}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)), kv("seen", seen) ]);

        if set_has(seen, x) {
            log(`${x} is already in the set -> duplicate`, "result");
            result("true");
            step("check", `${x} is already in the set — we have seen this value before.`,
                [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)), kv("seen", seen).hl([x]) ]);
            step("dup", "Return true and stop; one duplicate is all the question asks for.",
                [ array("nums", nums).hl([i]), kv("seen", seen).hl([x]) ], "return");
            leave("true");
            return;
        }

        step("check", `${x} is not in the set yet.`,
            [ array("nums", nums).ptr("i", i).done(before(i)), kv("seen", seen) ]);

        set_add(seen, x);
        set(#{ seen: SET(set_items(seen), [x]) });
        log(`add ${x}`);
        step("store", `Add ${x}. The set now holds ${set_len(seen)} distinct value(s).`,
            [ array("nums", nums).ptr("i", i).done(before(i)), kv("seen", seen).hl([x]) ]);
    }

    log("no duplicates", "result");
    result("false");
    step("none", `All ${nums.len()} values were distinct, so there is no duplicate.`,
        [ array("nums", nums).done(seq(0, nums.len() - 1)), kv("seen", seen) ], "return");
    leave("false");
}
