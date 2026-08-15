// Longest Consecutive Sequence — only start walking from a sequence's head.
//
// The set makes membership O(1), but the real trick is the `n-1 not present`
// test: it means each run is walked exactly once no matter where its members
// sit in the array, which is what keeps the whole thing linear.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let seen = set_new();

    enter("longestConsecutive", #{ nums: A(nums) });
    step("build", "Put every value in a set so membership costs nothing.",
        [ array("nums", nums), kv("set", seen) ]);

    for i in 0..n {
        step("add", `Add ${nums[i]}.`,
            [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)), kv("set", seen) ]);
        set_add(seen, nums[i]);
        set(#{ set: SET(set_items(seen)) });
        step("insert", `The set holds ${set_len(seen)} distinct value(s).`,
            [ array("nums", nums).ptr("i", i).done(before(i)), kv("set", seen).hl([nums[i]]) ]);
    }

    let best = 0;
    set(#{ best: N(best) });
    step("best", "No run measured yet.", [ array("nums", nums), kv("set", seen) ]);

    let members = set_items(seen);
    for idx in 0..members.len() {
        let v = members[idx];
        set(#{ v: N(v) });
        step("scan", `Consider ${v}.`, [ array("nums", nums), kv("set", seen).hl([v]) ]);

        if set_has(seen, v - 1) {
            step("isstart", `${v - 1} is also present, so ${v} sits in the middle of a run — whoever starts it will count it.`,
                [ kv("set", seen).hl([v - 1, v]) ]);
            continue;
        }
        step("isstart", `${v - 1} is missing, so ${v} is the head of a run. This is the only place we walk from.`,
            [ kv("set", seen).hl([v]) ]);

        let length = 1;
        set(#{ length: N(length) });
        step("len", `Run starts at ${v}, length 1.`, [ kv("set", seen).hl([v]) ]);

        let run = [v];
        while set_has(seen, v + length) {
            step("walk", `${v + length} is present too.`, [ kv("set", seen).hl(run + [v + length]) ]);
            run.push(v + length);
            length += 1;
            set(#{ length: N(length) });
            step("grow", `Run is now ${length} long: ${brackets(run)}.`, [ kv("set", seen).hl(run) ]);
        }

        if length > best {
            step("cmp", `${length} beats the best run so far (${best}).`, [ kv("set", seen).hl(run) ]);
            best = length;
            set(#{ best: N(best) });
            log(`new best run ${brackets(run)}`, "result");
            step("update", `Best is now ${best}.`, [ kv("set", seen).hl(run) ]);
        } else {
            step("cmp", `${length} does not beat ${best}.`, [ kv("set", seen) ]);
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("return", `The longest consecutive run is ${best} long — found in one pass over the set.`,
        [ array("nums", nums), kv("set", seen) ], "return");
    leave(`${best}`);
}
