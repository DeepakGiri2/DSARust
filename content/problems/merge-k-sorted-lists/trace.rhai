// Merge K Sorted Lists — pair up and halve, round by round.
//
// Merging into one accumulator would re-walk the early elements once per list:
// k passes over an ever-growing result. Pairing halves the count each round
// instead, so every element is touched log k times.

fn merge_two(a, b) {
    let out = [];
    let i = 0;
    let j = 0;
    while i < a.len() && j < b.len() {
        if a[i] <= b[j] { out.push(a[i]); i += 1; } else { out.push(b[j]); j += 1; }
    }
    while i < a.len() { out.push(a[i]); i += 1; }
    while j < b.len() { out.push(b[j]); j += 1; }
    out
}

fn labels(lists) {
    let out = [];
    for l in lists { out.push(join(l, ",")); }
    out
}

fn trace(input) {
    let lists = [];
    for spec in words(input.lists) {
        let l = [];
        for v in spec.split(",") { l.push(num(v)); }
        lists.push(l);
    }

    enter("mergeKLists", #{ k: N(lists.len()) });
    if lists.len() == 0 {
        step("empty", "Nothing to merge.", [ text("lists", []) ]);
        result("");
        step("retNil", "Return nil.", [ text("lists", []) ], "return");
        leave("");
        return;
    }
    step("empty", `${lists.len()} sorted list(s).`, [ text("lists", labels(lists)) ]);

    let round = 0;
    while lists.len() > 1 {
        round += 1;
        set(#{ round: N(round), lists: N(lists.len()) });
        step("round", `Round ${round}: ${lists.len()} lists become ${(lists.len() + 1) / 2}.`,
            [ text("lists", labels(lists)) ]);

        let merged = [];
        step("newRound", "Pair them up.", [ text("lists", labels(lists)) ]);

        let i = 0;
        while i < lists.len() {
            if i + 1 < lists.len() {
                step("pairs", `Merge list ${i} with list ${i + 1}.`,
                    [ text("lists", labels(lists)).hl([i, i + 1]) ]);
                step("hasPair", `[${join(lists[i], ",")}] and [${join(lists[i + 1], ",")}].`,
                    [ text("lists", labels(lists)).hl([i, i + 1]) ]);
                let m = merge_two(lists[i], lists[i + 1]);
                merged.push(m);
                log(`merged -> ${join(m, ",")}`);
                step("merge", `Gives [${join(m, ",")}].`,
                    [ text("merged so far", labels(merged)).hl([merged.len() - 1]) ]);
            } else {
                step("hasPair", `List ${i} has no partner this round, so it carries over unchanged.`,
                    [ text("lists", labels(lists)).hl([i]) ]);
                merged.push(lists[i]);
                step("merge", "Carried over.", [ text("merged so far", labels(merged)) ]);
            }
            i += 2;
        }

        lists = merged;
        step("replace", `${lists.len()} list(s) left.`, [ text("lists", labels(lists)) ]);
    }

    let answer = join(lists[0], " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `One sorted list remains: ${answer}.`,
        [ array("merged", lists[0]).hl(seq(0, lists[0].len() - 1)) ], "return");
    leave(answer);
}
