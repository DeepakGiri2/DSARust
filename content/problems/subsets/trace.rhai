// Subsets — the include/exclude decision tree, made visible.
//
// Every element is a fork: take it or leave it. The call stack panel *is* the
// path down that tree, and the recursion reaching the bottom is exactly one
// completed subset. 2^n leaves, 2^n subsets.

fn backtrack(nums, i, subset, found) {
    enter(`backtrack(i=${i})`, #{ i: N(i), subset: A(subset) });

    if i == nums.len() {
        step("base", `i has reached the end, so every element has been decided.`,
            [ array("nums", nums).done(seq(0, nums.len() - 1)),
              array("current subset", subset) ]);
        found.push(brackets(subset));
        log(`subset ${brackets(subset)}`, "result");
        step("add", `Record ${brackets(subset)} — that is subset number ${found.len()}.`,
            [ array("nums", nums).done(seq(0, nums.len() - 1)),
              array("current subset", subset),
              text("subsets so far", found) ]);
        step("ret", "Return and unwind to the last undecided element.",
            [ text("subsets so far", found) ], "return");
        leave("");
        return found;
    }

    step("base", `Element ${i} (${nums[i]}) has not been decided yet.`,
        [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)),
          array("current subset", subset) ]);

    subset.push(nums[i]);
    step("include", `Branch 1: take ${nums[i]}.`,
        [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)),
          array("current subset", subset).hl([subset.len() - 1]) ]);
    step("recIn", `Recurse with ${nums[i]} included.`,
        [ array("nums", nums).ptr("i", i).done(before(i)),
          array("current subset", subset) ], "call");
    found = backtrack(nums, i + 1, subset, found);

    subset.truncate(subset.len() - 1);
    step("exclude",
        `Back at element ${i}. Undo the choice — this is the "backtrack" — and try branch 2: leave ${nums[i]} out.`,
        [ array("nums", nums).ptr("i", i).bad([i]).done(before(i)),
          array("current subset", subset) ]);
    step("recEx", `Recurse with ${nums[i]} excluded.`,
        [ array("nums", nums).ptr("i", i).done(before(i)),
          array("current subset", subset) ], "call");
    found = backtrack(nums, i + 1, subset, found);

    leave("");
    found
}

fn trace(input) {
    let nums = input.nums;
    let found = [];

    enter("subsets", #{ nums: A(nums) });
    step("init", "Collect every subset here.", [ array("nums", nums) ]);
    step("sub", "The working subset starts empty.",
        [ array("nums", nums), array("current subset", []) ]);
    step("call", "Start the decision tree at element 0.",
        [ array("nums", nums).ptr("i", 0) ], "call");

    found = backtrack(nums, 0, [], found);

    log(`${found.len()} subsets`, "result");
    result(`${found.len()} subsets`);
    step("done",
        `${nums.len()} elements, each with two choices, gives ${found.len()} subsets — including the empty one and the full set.`,
        [ text("subsets", found) ], "return");
    leave(`${found.len()} subsets`);
}
