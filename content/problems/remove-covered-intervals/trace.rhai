fn parse_pairs(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

fn labels(ivs) {
    let out = [];
    for iv in ivs { out.push(`[${iv[0]},${iv[1]}]`); }
    out
}

fn sort_by_start(ivs) {
    for i in 1..ivs.len() {
        let j = i;
        while j > 0 && ivs[j - 1][0] > ivs[j][0] {
            swap_at(ivs, j - 1, j);
            j -= 1;
        }
    }
    ivs
}

// Remove Covered Intervals — the tie-break is the whole trick.
//
// After sorting by start, every later interval starts at or after every
// earlier one, so containment turns on the end alone: an interval is covered
// precisely when its end does not go beyond the furthest end already seen.
//
// The descending tie-break on the end handles the one case that would
// otherwise break it. Two intervals with the same start are nested; sorting
// the longer one first means it becomes the coverer, whereas the other order
// would make it look like the *first* of a pair and let both survive.

fn sort_covered(ivs) {
    for i in 1..ivs.len() {
        let j = i;
        while j > 0 && (ivs[j - 1][0] > ivs[j][0] ||
                        (ivs[j - 1][0] == ivs[j][0] && ivs[j - 1][1] < ivs[j][1])) {
            swap_at(ivs, j - 1, j);
            j -= 1;
        }
    }
    ivs
}

fn trace(input) {
    let ivs = parse_pairs(input.intervals);
    let n = ivs.len();

    enter("removeCoveredIntervals", #{ n: N(n) });

    ivs = sort_covered(ivs);
    step("sort", "Start ascending, end descending on a tie — so where two intervals share a start, the longer one comes first and does the covering.",
        [ array("intervals (sorted)", labels(ivs)) ]);

    let count = 0;
    let reach = 0;
    set(#{ count: N(count), reach: N(reach) });
    step("init", "Survivors so far, and the furthest end any of them reaches.",
        [ text("state", [`kept: ${count}`, `furthest end: ${reach}`]) ]);

    let kept = [];
    let dropped = [];
    for i in 0..n {
        let iv = ivs[i];
        step("loop", `Interval ${i}: [${iv[0]},${iv[1]}].`,
            [ array("intervals (sorted)", labels(ivs)).ptr("i", i).done(kept).bad(dropped),
              text("state", [`furthest end: ${reach}`]) ]);

        if iv[1] > reach {
            step("extends", `It ends at ${iv[1]}, beyond the ${reach} reached so far — so nothing seen already can contain it.`,
                [ array("intervals (sorted)", labels(ivs)).ptr("i", i).hl([i]) ]);
            count += 1;
            kept.push(i);
            set(#{ count: N(count) });
            log(`keep [${iv[0]},${iv[1]}]`);
            step("keep", `Keep it: ${count} so far.`,
                [ array("intervals (sorted)", labels(ivs)).done(kept).bad(dropped) ]);
            reach = iv[1];
            set(#{ reach: N(reach) });
            step("extend", `Furthest end is now ${reach}.`,
                [ text("state", [`kept: ${count}`, `furthest end: ${reach}`]) ]);
        } else {
            dropped.push(i);
            log(`[${iv[0]},${iv[1]}] is covered`);
            step("covered", `It ends at ${iv[1]}, no further than ${reach}. Its start is also no earlier than the interval that reached ${reach}, so it sits entirely inside it.`,
                [ array("intervals (sorted)", labels(ivs)).done(kept).bad(dropped) ]);
        }
    }

    log(`answer ${count}`, "result");
    result(`${count}`);
    step("ret", `${plural(count, "interval")} remain.`,
        [ array("intervals (sorted)", labels(ivs)).done(kept).bad(dropped) ], "return");
    leave(`${count}`);
}
