// Merge Triplets — merging is irreversible, so one bad value poisons a triplet.
//
// The merge keeps the larger value in every position, so nothing ever goes
// down. A triplet holding anything above the target can therefore never be
// used: merging it would push that position past the target permanently.
// Once those are discarded, no remaining triplet can do harm, so they may all
// be merged — and the question collapses to whether each target value appears
// somewhere among them.

fn parse_triples(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 3 { out.push([num(w[0]), num(w[1]), num(w[2])]); }
    }
    out
}

fn labels(triplets) {
    let out = [];
    for t in triplets { out.push(`[${t[0]},${t[1]},${t[2]}]`); }
    out
}

fn found_view(found) {
    let out = [];
    for i in 0..3 { out.push(if found[i] { "yes" } else { "no" }); }
    out
}

fn trace(input) {
    let triplets = parse_triples(input.triplets);
    let tw = words(input.target);
    let target = [num(tw[0]), num(tw[1]), num(tw[2])];
    let n = triplets.len();

    enter("mergeTriplets", #{});

    let found = [false, false, false];
    step("init", `The target is [${join(target, ",")}]. Each position needs some usable triplet that already holds exactly that value.`,
        [ array("triplets", labels(triplets)),
          array("target reached in each position?", found_view(found)) ]);

    let usable = [];
    let poison = [];
    for k in 0..n {
        let t = triplets[k];
        step("loop", `Triplet ${k}: [${join(t, ",")}].`,
            [ array("triplets", labels(triplets)).ptr("k", k).done(usable).bad(poison) ]);

        if t[0] > target[0] || t[1] > target[1] || t[2] > target[2] {
            poison.push(k);
            step("tooBig", `Something in it exceeds the target, and merging never brings a value back down — so using it would overshoot for good.`,
                [ array("triplets", labels(triplets)).ptr("k", k).bad(poison),
                  text("target", [`[${join(target, ",")}]`]) ]);
            continue;
        }

        usable.push(k);
        step("check", "Nothing in it exceeds the target, so merging it in can do no harm at all.",
            [ array("triplets", labels(triplets)).ptr("k", k).done(usable).bad(poison) ]);

        for i in 0..3 {
            if t[i] == target[i] {
                step("match", `Position ${i} holds exactly ${target[i]}.`,
                    [ array("triplets", labels(triplets)).ptr("k", k).done(usable) ]);
                found[i] = true;
                log(`position ${i} satisfied by triplet ${k}`);
                step("mark", `So position ${i} is achievable.`,
                    [ array("target reached in each position?", found_view(found)).hl([i]) ]);
            }
        }
    }

    let ok = found[0] && found[1] && found[2];
    log(`answer ${ok}`, "result");
    result(`${ok}`);
    step("ret",
        if ok {
            `Every position is covered by some usable triplet, and merging all ${usable.len()} of them together hits the target exactly.`
        } else {
            "At least one position has no usable triplet holding its target value, so no sequence of merges can reach it."
        },
        [ array("triplets", labels(triplets)).done(usable).bad(poison),
          array("target reached in each position?", found_view(found)) ], "return");
    leave(`${ok}`);
}
