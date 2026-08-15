// Arranging Coins — search the number of rows, not the coins.
//
// Laying rows out one at a time is linear in the answer. But the cost of k
// rows has a closed form, k(k+1)/2, and that cost only increases — so "k rows
// fit" switches from true to false exactly once. That is the licence to halve.

fn staircase(k, cap) {
    let out = [];
    let r = 1;
    while r <= k && r <= cap { out.push(r); r += 1; }
    out
}

fn trace(input) {
    let n = input.n;

    enter("arrangeCoins", #{ n: N(n) });

    let lo = 0;
    let hi = n;
    let best = 0;
    set(#{ lo: N(lo), hi: N(hi) });
    step("bounds", `${n} coins can fill somewhere between 0 and ${n} rows.`,
        [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);
    set(#{ best: N(best) });
    step("best", "No row count is known to fit yet.",
        [ text("search range", [`lo = ${lo}`, `hi = ${hi}`, `best = ${best}`]) ]);

    while lo <= hi {
        step("loop", `Row counts ${lo}..${hi} are still possible.`,
            [ text("search range", [`lo = ${lo}`, `hi = ${hi}`, `best so far = ${best}`]) ]);

        let k = lo + (hi - lo) / 2;
        set(#{ k: N(k) });
        step("mid", `Try ${k} complete rows.`,
            [ bars("coins per row (1, 2, 3, ...)", staircase(k, 12)),
              text("search range", [`lo = ${lo}`, `k = ${k}`, `hi = ${hi}`]) ]);

        let need = k * (k + 1) / 2;
        step("need", `${k} rows need 1 + 2 + ... + ${k} = ${k}·${k + 1}/2 = ${need} coins.`,
            [ bars("coins per row (1, 2, 3, ...)", staircase(k, 12)),
              text("test", [`rows: ${k}`, `coins needed: ${need}`, `coins available: ${n}`]) ]);

        if need <= n {
            step("fits", `${need} ≤ ${n}, so ${k} rows fit — and so does every smaller count, which is why nothing below needs checking.`,
                [ text("test", [`${need} ≤ ${n}`, "fits"]) ]);
            best = k;
            set(#{ best: N(best) });
            log(`${k} rows fit`);
            step("keep", `Remember ${best} and try for more.`,
                [ text("search range", [`best = ${best}`]) ]);
            lo = k + 1;
            set(#{ lo: N(lo) });
            step("more", `lo = ${lo}.`,
                [ text("search range", [`lo = ${lo}`, `hi = ${hi}`, `best = ${best}`]) ]);
        } else {
            step("tooMany", `${need} > ${n}, so ${k} rows cannot be filled — nor can any larger number.`,
                [ text("test", [`${need} > ${n}`, "too many"]) ]);
            hi = k - 1;
            set(#{ hi: N(hi) });
            step("fewer", `hi = ${hi}.`,
                [ text("search range", [`lo = ${lo}`, `hi = ${hi}`, `best = ${best}`]) ]);
        }
    }

    let used = best * (best + 1) / 2;
    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `${best} complete rows use ${used} coins, leaving ${n - used} — not enough for a row of ${best + 1}.`,
        [ bars("coins per row (1, 2, 3, ...)", staircase(best, 12)),
          text("answer", [`complete rows: ${best}`, `coins used: ${used} of ${n}`]) ], "return");
    leave(`${best}`);
}
