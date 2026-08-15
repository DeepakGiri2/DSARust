// Sqrt(x) — searching the answer, not an array.
//
// There is no data to index; the thing being halved is the range of possible
// answers. What licenses that is monotonicity: k·k ≤ x holds for every k up to
// the answer and fails for every k after it, so one test at the midpoint always
// discards a whole half. The `best` variable is what turns "some k that fits"
// into "the largest k that fits".

fn trace(input) {
    let x = input.x;

    enter("mySqrt", #{ x: N(x) });

    let lo = 0;
    let hi = x;
    let best = 0;
    set(#{ lo: N(lo), hi: N(hi) });
    step("bounds", `The answer is somewhere in 0..${x}.`,
        [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);
    set(#{ best: N(best) });
    step("best", "No qualifying value found yet.",
        [ text("search range", [`lo = ${lo}`, `hi = ${hi}`, `best = ${best}`]) ]);

    while lo <= hi {
        step("loop", `Candidates ${lo}..${hi} are still possible.`,
            [ text("search range", [`lo = ${lo}`, `hi = ${hi}`, `best so far = ${best}`]) ]);

        let mid = lo + (hi - lo) / 2;
        set(#{ mid: N(mid) });
        step("mid", `Try ${mid}.`,
            [ text("search range", [`lo = ${lo}`, `mid = ${mid}`, `hi = ${hi}`]) ]);

        let sq = mid * mid;
        step("square", `${mid} × ${mid} = ${sq}.`,
            [ text("test", [`${mid}² = ${sq}`, `x = ${x}`]) ]);

        if sq <= x {
            step("fits", `${sq} ≤ ${x}, so ${mid} qualifies — and so does everything below it, which is why nothing under ${mid} needs testing.`,
                [ text("test", [`${mid}² = ${sq} ≤ ${x}`, "qualifies"]) ]);
            best = mid;
            set(#{ best: N(best) });
            log(`${mid} fits`);
            step("keep", `Remember ${best}: it is the largest qualifying value seen so far.`,
                [ text("search range", [`best = ${best}`]) ]);
            lo = mid + 1;
            set(#{ lo: N(lo) });
            step("up", `Reach higher: lo = ${lo}.`,
                [ text("search range", [`lo = ${lo}`, `hi = ${hi}`, `best = ${best}`]) ]);
        } else {
            step("over", `${sq} > ${x}, so ${mid} is too big — and so is everything above it.`,
                [ text("test", [`${mid}² = ${sq} > ${x}`, "too big"]) ]);
            hi = mid - 1;
            set(#{ hi: N(hi) });
            step("down", `Come down: hi = ${hi}.`,
                [ text("search range", [`lo = ${lo}`, `hi = ${hi}`, `best = ${best}`]) ]);
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `${best}² = ${best * best} ≤ ${x} < ${(best + 1) * (best + 1)} = ${best + 1}², so ${best} is the integer square root.`,
        [ text("answer", [`sqrt(${x}) = ${best}`]) ], "return");
    leave(`${best}`);
}
