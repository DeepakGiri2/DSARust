// Ship Within D Days — binary search over answers, greedy to test one.
//
// Two ideas stacked. First: capacities are monotone — if a ship of size c can
// do it, so can every larger ship — so the set of workable capacities is a
// suffix of the number line and can be halved. Second: testing one capacity
// needs no cleverness, because holding a package back can never save a day.
// Load until it would overflow, then start a new day.

fn feasible(weights, days, capacity) {
    enter(`feasible(capacity = ${capacity})`, #{ capacity: N(capacity) });

    let need = 1;
    let load = 0;
    let splits = [];
    step("start", "Start on day 1 with an empty hold.",
        [ array("weights", weights),
          text("shipping", [`capacity: ${capacity}`, `day: 1`, `load: 0`]) ]);

    for i in 0..weights.len() {
        let w = weights[i];
        step("scan", `Package ${i} weighs ${w}; the hold currently holds ${load}.`,
            [ array("weights", weights).ptr("i", i).done(before(i)).bad(splits),
              text("shipping", [`capacity: ${capacity}`, `day: ${need}`, `load: ${load}`]) ]);

        if load + w > capacity {
            step("overflow", `${load} + ${w} = ${load + w} exceeds ${capacity}, so it cannot go on today's ship.`,
                [ array("weights", weights).ptr("i", i).bad([i]),
                  text("shipping", [`capacity: ${capacity}`, `day: ${need}`, `load: ${load}`]) ]);
            need += 1;
            load = 0;
            splits.push(i);
            step("newDay", `Sail, and start day ${need} empty. Holding the package back instead could not possibly help — it would still have to ship eventually.`,
                [ array("weights", weights).ptr("i", i).bad(splits),
                  text("shipping", [`capacity: ${capacity}`, `day: ${need}`, `load: 0`]) ]);
        }

        load += w;
        step("add", `Load it: the hold now holds ${load}.`,
            [ array("weights", weights).ptr("i", i).done(before(i + 1)).bad(splits),
              text("shipping", [`capacity: ${capacity}`, `day: ${need}`, `load: ${load}`]) ]);
    }

    let ok = need <= days;
    log(`capacity ${capacity} needs ${need} day(s)`);
    step("verdict", `A capacity of ${capacity} needs ${plural(need, "day")}, against a budget of ${days} — ${if ok { "workable" } else { "too slow" }}.`,
        [ array("weights", weights).bad(splits),
          text("shipping", [`capacity: ${capacity}`, `days needed: ${need}`, `days allowed: ${days}`]) ],
        "return");
    leave(`${ok}`);
    ok
}

fn trace(input) {
    let weights = input.weights;
    let days = input.days;
    let n = weights.len();

    enter("shipWithinDays", #{ days: N(days) });

    let lo = 0;
    let hi = 0;
    step("bounds", "The answer has to lie between two obvious extremes.",
        [ array("weights", weights) ]);

    for i in 0..n {
        if weights[i] > lo { lo = weights[i]; }
        hi += weights[i];
    }
    set(#{ lo: N(lo), hi: N(hi) });
    step("span", `No ship smaller than the heaviest package (${lo}) could ever sail, and a ship of ${hi} carries everything in one day. So the answer is in [${lo}..${hi}].`,
        [ array("weights", weights),
          text("search range", [`lo = ${lo} (heaviest package)`, `hi = ${hi} (total weight)`]) ]);

    while lo < hi {
        step("loop", `Capacities ${lo}..${hi} are still possible.`,
            [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);

        let mid = lo + (hi - lo) / 2;
        set(#{ mid: N(mid) });
        step("mid", `Test a capacity of ${mid}.`,
            [ text("search range", [`lo = ${lo}`, `mid = ${mid}`, `hi = ${hi}`]) ]);

        step("check", `Can a ship of ${mid} finish within ${days} day(s)?`,
            [ array("weights", weights) ], "call");
        let ok = feasible(weights, days, mid);

        if ok {
            hi = mid;
            set(#{ hi: N(hi) });
            step("smaller", `Yes. So every capacity above ${mid} works too and is wasteful — keep ${mid} as a candidate and search below it.`,
                [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);
        } else {
            step("fail", `No — ${mid} is too small.`,
                [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);
            lo = mid + 1;
            set(#{ lo: N(lo) });
            step("larger", `Then nothing at or below ${mid} works either: lo = ${lo}.`,
                [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);
        }
    }

    log(`answer ${lo}`, "result");
    result(`${lo}`);
    step("ret", `${lo} is the smallest capacity that finishes in ${days} day(s) — ${lo - 1} does not.`,
        [ array("weights", weights), text("answer", [`capacity: ${lo}`]) ], "return");
    leave(`${lo}`);
}
