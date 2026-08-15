// Combination Sum II — sorting pays for both the dedup and the pruning.
//
// The duplicate rule is subtle: `i > start` means "this is not the first
// choice at this level". The first copy of a repeated value at a level already
// explores every combination the later copies could produce, so skipping them
// removes exactly the duplicates and nothing else — note that a repeat is
// still allowed *deeper* in the recursion, which is how {1,1,6} survives.
//
// And because the values are increasing, one that overshoots means every later
// one overshoots too: `break`, not `continue`.

fn back(cands, start, remain, cur, res) {
    enter(`back(start = ${start}, remain = ${remain})`, #{ start: N(start), remain: N(remain) });

    if remain == 0 {
        step("hit", "The remaining target is exactly zero.",
            [ array("candidates (sorted)", cands), array("current combination", cur) ]);
        res.push(join(cur, ","));
        log(`found ${join(cur, ",")}`, "result");
        step("record", `Record {${join(cur, ", ")}}.`, [ text("results", res) ]);
        step("retHit", "Unwind.", [ text("results", res) ], "return");
        leave("");
        return res;
    }

    let i = start;
    while i < cands.len() {
        step("loop", `Considering candidate ${i} (value ${cands[i]}); ${remain} still to find.`,
            [ array("candidates (sorted)", cands).ptr("i", i).done(before(start)),
              array("current combination", cur) ]);

        if i > start && cands[i] == cands[i - 1] {
            step("dup", `${cands[i]} is the same value as the previous candidate *at this level*. The first copy already explored everything this one could, so skipping it removes a duplicate combination — not a valid one.`,
                [ array("candidates (sorted)", cands).ptr("i", i).bad([i]) ]);
            i += 1;
            continue;
        }

        if cands[i] > remain {
            step("tooBig", `${cands[i]} is more than the ${remain} still needed — and the list is sorted, so every later candidate is too big as well. Stop, rather than skip.`,
                [ array("candidates (sorted)", cands).ptr("i", i).bad(seq(i, cands.len() - 1)) ]);
            break;
        }

        cur.push(cands[i]);
        step("choose", `Take ${cands[i]}; ${remain - cands[i]} left to find.`,
            [ array("candidates (sorted)", cands).ptr("i", i).hl([i]),
              array("current combination", cur) ]);

        step("recurse", `Continue from candidate ${i + 1} — each number may be used only once, so the next level starts past this one.`,
            [ array("current combination", cur) ], "call");
        res = back(cands, i + 1, remain - cands[i], cur, res);

        cur.truncate(cur.len() - 1);
        step("undo", `Put ${cands[i]} back.`, [ array("current combination", cur) ]);

        i += 1;
    }

    leave("");
    res
}

fn trace(input) {
    let cands = input.candidates;
    let target = input.target;

    enter("combinationSum2", #{ target: N(target) });

    for i in 1..cands.len() {
        let j = i;
        while j > 0 && cands[j - 1] > cands[j] {
            swap_at(cands, j - 1, j);
            j -= 1;
        }
    }
    step("sort", "Sorting groups equal values together — needed for the duplicate rule — and makes the values increase, which is what allows an early break.",
        [ array("candidates (sorted)", cands) ]);

    step("init", "The unique combinations found.", [ text("results", []) ]);

    step("call", `Start with nothing chosen and ${target} to find.`,
        [ array("candidates (sorted)", cands) ], "call");
    let res = back(cands, 0, target, [], []);

    let answer = join(res, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(res.len(), "unique combination")}.`, [ text("results", res) ], "return");
    leave(answer);
}
