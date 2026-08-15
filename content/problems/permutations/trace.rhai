// Permutations — choose, recurse, un-choose.
//
// The `used` flags are what make this a permutation rather than a combination:
// every position may take any element that is not already in the current
// arrangement. Watch a flag go on before the recursive call and off again
// straight after — that pairing is the backtracking.

fn backtrack(nums, cur, used, found) {
    enter(`backtrack(depth=${cur.len()})`, #{ current: A(cur) });

    if cur.len() == nums.len() {
        found.push(brackets(cur));
        log(`found ${brackets(cur)}`, "result");
        step("base", "Every position is filled.",
            [ array("nums", nums), array("current", cur).hl(seq(0, cur.len() - 1)) ]);
        step("add", `Record ${brackets(cur)} — permutation ${found.len()}.`,
            [ array("current", cur), text("permutations", found) ]);
        step("ret", "Unwind to try a different choice.", [ text("permutations", found) ], "return");
        leave("");
        return found;
    }

    step("base", `Position ${cur.len()} still needs a value.`,
        [ array("nums", nums).done(marked(used)), array("current", cur) ]);

    for i in 0..nums.len() {
        step("loop", `Could ${nums[i]} go here?`,
            [ array("nums", nums).ptr("i", i).done(marked(used)), array("current", cur) ]);

        if used[i] {
            step("skip", `${nums[i]} is already in the arrangement, so it cannot be used twice.`,
                [ array("nums", nums).ptr("i", i).bad([i]).done(marked(used)),
                  array("current", cur) ]);
            continue;
        }

        used[i] = true;
        step("mark", `Mark ${nums[i]} as used.`,
            [ array("nums", nums).ptr("i", i).hl([i]).done(marked(used)),
              array("current", cur) ]);
        cur.push(nums[i]);
        step("choose", `Place ${nums[i]} at position ${cur.len() - 1}.`,
            [ array("nums", nums).done(marked(used)),
              array("current", cur).hl([cur.len() - 1]) ]);

        step("rec", "Fill the next position.",
            [ array("nums", nums).done(marked(used)), array("current", cur) ], "call");
        found = backtrack(nums, cur, used, found);

        cur.truncate(cur.len() - 1);
        step("unchoose", `Take ${nums[i]} back out of the arrangement.`,
            [ array("nums", nums).done(marked(used)), array("current", cur) ]);
        used[i] = false;
        step("unmark", `Free it up again — that is what lets it appear in a later branch.`,
            [ array("nums", nums).ptr("i", i).done(marked(used)), array("current", cur) ]);
    }

    leave("");
    found
}

/// Indices currently flagged as used, for dimming.
fn marked(used) {
    let out = [];
    for i in 0..used.len() { if used[i] { out.push(i); } }
    out
}

fn trace(input) {
    let nums = input.nums;
    let used = [];
    for i in 0..nums.len() { used.push(false); }

    enter("permute", #{ nums: A(nums) });
    step("init", "Collect every arrangement.", [ array("nums", nums) ]);
    step("cur", "The working arrangement starts empty.",
        [ array("nums", nums), array("current", []) ]);
    step("used", "No element is used yet.", [ array("nums", nums) ]);
    step("call", "Fill position 0 first.", [ array("nums", nums) ], "call");

    let found = backtrack(nums, [], used, []);

    log(`${found.len()} permutations`, "result");
    result(`${found.len()} permutations`);
    step("done", `${nums.len()}! = ${found.len()} arrangements, each built by choosing, recursing and undoing.`,
        [ text("permutations", found) ], "return");
    leave(`${found.len()} permutations`);
}
