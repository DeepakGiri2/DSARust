// Hand of Straights — the smallest card has no options, so follow it.
//
// A group is a run of consecutive values, so any card must sit somewhere in
// such a run. The smallest remaining card cannot be anywhere but the *start*
// of one, because nothing below it survives to precede it. That removes all
// choice: build runs from the lowest value outwards, as many at a time as
// there are copies of it. A shortfall anywhere in the run is fatal, since the
// card that ran out had no other placement either.

fn sorted_keys(count) {
    let out = [];
    for e in count { out.push(e[0]); }
    for i in 1..out.len() {
        let j = i;
        while j > 0 && out[j - 1] > out[j] {
            swap_at(out, j - 1, j);
            j -= 1;
        }
    }
    out
}

fn counts_view(count, keys) {
    let out = [];
    for k in keys { out.push(`${k}×${map_get(count, k)}`); }
    out
}

fn trace(input) {
    let hand = input.hand;
    let size = input.groupSize;
    let n = hand.len();

    enter("isNStraightHand", #{ groupSize: N(size) });

    if n % size != 0 {
        log(`${n} cards cannot split into groups of ${size}`, "result");
        step("divisible", `${n} cards cannot be divided into groups of ${size} at all.`,
            [ array("hand", hand), text("counts", [`cards: ${n}`, `group size: ${size}`]) ], "return");
        result("false");
        leave("false");
        return;
    }
    step("divisible", `${n} cards divide evenly into ${n / size} group(s) of ${size}.`,
        [ array("hand", hand) ]);

    let count = map_new();
    for c in hand { map_inc(count, c, 1); }
    let keys = sorted_keys(count);
    step("count", "How many of each value are in hand.",
        [ array("card values (ascending)", keys),
          array("copies remaining", counts_view(count, keys)) ]);
    step("sort", "Sorted ascending, so the lowest unspent value is always found first.",
        [ array("card values (ascending)", keys),
          array("copies remaining", counts_view(count, keys)) ]);

    let ok = true;
    for ki in 0..keys.len() {
        let c = keys[ki];
        step("loop", `Lowest unexamined value: ${c}.`,
            [ array("card values (ascending)", keys).ptr("c", ki),
              array("copies remaining", counts_view(count, keys)) ]);

        let need = map_get(count, c);
        step("need", `${plural(need, "copy")} of ${c} remain${if need == 1 { "s" } else { "" }}.`,
            [ array("copies remaining", counts_view(count, keys)).hl([ki]) ]);

        if need == 0 {
            step("spent", `All the ${c}s were already absorbed into earlier groups.`,
                [ array("copies remaining", counts_view(count, keys)) ]);
            continue;
        }

        for k in 0..size {
            let v = c + k;
            let have = if map_has(count, v) { map_get(count, v) } else { 0 };
            step("build", `Each of the ${need} group(s) starting at ${c} needs a ${v}; there ${if have == 1 { "is" } else { "are" }} ${have}.`,
                [ array("copies remaining", counts_view(count, keys)),
                  text("building", [`${need} group(s) from ${c}`, `need ${need} × ${v}`]) ]);

            if have < need {
                ok = false;
                log(`not enough ${v}s`, "result");
                step("missing", `Only ${have} of them. Since ${c} could not start anywhere else, and each of those groups needs a ${v}, no arrangement exists.`,
                    [ array("copies remaining", counts_view(count, keys)) ]);
                step("no", "Impossible.", [ text("answer", ["false"]) ], "return");
                break;
            }

            map_inc(count, v, -need);
            log(`use ${need} × ${v}`);
            step("consume", `Take ${need} of them; ${map_get(count, v)} left.`,
                [ array("copies remaining", counts_view(count, keys)) ]);
        }

        if !ok { break; }
    }

    if ok {
        log("every card placed", "result");
        step("yes", "Every card was absorbed into a run, and no choice was ever made — so this is the only possible arrangement.",
            [ array("copies remaining", counts_view(count, keys)) ], "return");
    }

    result(`${ok}`);
    leave(`${ok}`);
}
