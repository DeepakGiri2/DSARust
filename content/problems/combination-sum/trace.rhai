// Combination Sum — include or skip, with reuse allowed.
//
// Two rules keep the search from producing duplicates: recursing on the *same*
// index after including a candidate (so it can be reused), and moving strictly
// forward when skipping (so no earlier candidate ever comes back).

fn backtrack(cands, target, i, cur, total, found) {
    enter(`backtrack(i=${i}, left=${target - total})`, #{ i: N(i), total: N(total) });

    if total == target {
        found.push(brackets(cur));
        log(`found ${brackets(cur)}`, "result");
        step("base", `The running total has hit ${target} exactly.`,
            [ array("candidates", cands).ptr("i", i), array("current", cur) ]);
        step("add", `Record ${brackets(cur)}.`,
            [ array("current", cur).hl(seq(0, cur.len() - 1)), text("combinations", found) ]);
        step("ret", "Unwind and try something else.", [ text("combinations", found) ], "return");
        leave("");
        return found;
    }

    if total > target || i >= cands.len() {
        let why = if total > target {
            `${total} has overshot ${target}`
        } else {
            "the candidates have run out"
        };
        step("prune", `Give up on this branch: ${why}.`,
            [ array("candidates", cands).ptr("i", i).bad(if i < cands.len() { [i] } else { [] }),
              array("current", cur) ]);
        step("pret", "Return without recording anything.", [ array("current", cur) ], "return");
        leave("");
        return found;
    }

    step("base", `Running total ${total}, target ${target}, considering candidate ${cands[i]}.`,
        [ array("candidates", cands).ptr("i", i).hl([i]), array("current", cur) ]);

    cur.push(cands[i]);
    step("include", `Branch 1: take ${cands[i]}. Total becomes ${total + cands[i]}.`,
        [ array("candidates", cands).ptr("i", i).hl([i]),
          array("current", cur).hl([cur.len() - 1]) ]);
    step("recIn", `Recurse on the *same* index — ${cands[i]} may be used again.`,
        [ array("candidates", cands).ptr("i", i), array("current", cur) ], "call");
    found = backtrack(cands, target, i, cur, total + cands[i], found);

    cur.truncate(cur.len() - 1);
    step("exclude", `Undo that choice and try branch 2: skip ${cands[i]} for good.`,
        [ array("candidates", cands).ptr("i", i).bad([i]), array("current", cur) ]);
    step("recEx", `Recurse on index ${i + 1}, so ${cands[i]} can never reappear and no combination is produced twice.`,
        [ array("candidates", cands).ptr("i", i + 1), array("current", cur) ], "call");
    found = backtrack(cands, target, i + 1, cur, total, found);

    leave("");
    found
}

fn trace(input) {
    let cands = input.candidates;
    let target = input.target;
    let found = [];

    enter("combinationSum", #{ candidates: A(cands), target: N(target) });
    step("init", "Collect every combination that sums to the target.",
        [ array("candidates", cands) ]);
    step("cur", "The working combination starts empty.",
        [ array("candidates", cands), array("current", []) ]);
    step("call", "Start at candidate 0 with a total of 0.",
        [ array("candidates", cands).ptr("i", 0) ], "call");

    found = backtrack(cands, target, 0, [], 0, found);

    let answer = `[${join(found, ", ")}]`;
    log(`${found.len()} combination(s)`, "result");
    result(answer);
    step("done", `${plural(found.len(), "combination")} reach ${target}: ${answer}.`,
        [ text("combinations", found) ], "return");
    leave(answer);
}
