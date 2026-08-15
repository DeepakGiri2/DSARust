// Koko Eating Bananas — binary search over the answer, not the input.
//
// The array is never searched. What is searched is the range of possible
// speeds, and the reason that works is monotonicity: if speed k finishes in
// time then so does every larger speed. So the speeds form a block of "too
// slow" followed by a block of "fast enough", and binary search finds the
// boundary.

fn feasibility_row(piles, k) {
    let out = [];
    for p in piles { out.push((p + k - 1) / k); }
    out
}

fn trace(input) {
    let piles = input.piles;
    let h = input.h;
    let n = piles.len();

    enter("minEatingSpeed", #{ h: N(h) });
    let lo = 1;
    let hi = 0;
    set(#{ lo: N(lo), hi: N(hi) });
    step("init", "The slowest useful speed is 1.", [ bars("piles", piles) ]);

    for i in 0..n {
        step("maxLoop", `Pile ${i} holds ${piles[i]}.`,
            [ bars("piles", piles).ptr("i", i).done(before(i)) ]);
        if piles[i] > hi {
            hi = piles[i];
            set(#{ hi: N(hi) });
            step("setMax", `Eating ${hi} per hour clears any pile in one hour, so no faster speed is ever needed.`,
                [ bars("piles", piles).ptr("i", i).hl([i]) ]);
        }
    }

    while lo < hi {
        step("loop", `The answer is somewhere in speeds [${lo}..${hi}].`,
            [ bars("piles", piles),
              text("speed search", [`too slow: below ${lo}`, `fast enough: above ${hi}`,
                                    `candidates: ${lo}..${hi}`]) ]);

        let k = lo + (hi - lo) / 2;
        set(#{ k: N(k) });
        step("mid", `Try k = ${k} bananas per hour.`,
            [ bars("piles", piles), array(`hours needed at k = ${k}`, feasibility_row(piles, k)) ]);

        let hours = 0;
        set(#{ hours: N(hours) });
        step("count", "Add up the hours, one pile at a time.",
            [ array(`hours needed at k = ${k}`, feasibility_row(piles, k)) ]);

        for i in 0..n {
            let need = (piles[i] + k - 1) / k;
            hours += need;
            set(#{ hours: N(hours) });
            step("sum", `Pile ${i} has ${piles[i]}, so it takes ceil(${piles[i]}/${k}) = ${need} hour(s).`,
                [ bars("piles", piles).ptr("i", i).hl([i]),
                  array(`hours needed at k = ${k}`, feasibility_row(piles, k)).hl([i]) ]);
            step("add", `Running total: ${hours}.`,
                [ array(`hours needed at k = ${k}`, feasibility_row(piles, k)).done(before(i + 1)) ]);
        }

        if hours <= h {
            step("feasible", `${hours} hour(s) is within the ${h} allowed — k = ${k} works, and so does anything faster.`,
                [ array(`hours needed at k = ${k}`, feasibility_row(piles, k))
                    .hl(seq(0, n - 1)) ]);
            hi = k;
            set(#{ hi: N(hi) });
            step("shrink", `Keep ${k} as a candidate and look for something slower: hi = ${hi}.`,
                [ text("speed search", [`candidates: ${lo}..${hi}`]) ]);
        } else {
            step("feasible", `${hours} hour(s) exceeds ${h} — k = ${k} is too slow, and so is everything below it.`,
                [ array(`hours needed at k = ${k}`, feasibility_row(piles, k))
                    .bad(seq(0, n - 1)) ]);
            lo = k + 1;
            set(#{ lo: N(lo) });
            step("grow", `lo = ${lo}.`, [ text("speed search", [`candidates: ${lo}..${hi}`]) ]);
        }
    }

    log(`answer ${lo}`, "result");
    result(`${lo}`);
    step("ret", `${lo} is the slowest speed that still finishes in ${h} hour(s).`,
        [ bars("piles", piles), array(`hours needed at k = ${lo}`, feasibility_row(piles, lo)) ],
        "return");
    leave(`${lo}`);
}
