// Valid Perfect Square — an exact question, so the loop may exit early.
//
// The ladder of squares is increasing, which is the only reason halving is
// sound: one comparison rules out everything on one side. Unlike the integer
// square root there is no "best so far" to carry, because only an exact hit
// counts — and an exact hit ends the search immediately.

fn trace(input) {
    let num = input.num;

    enter("isPerfectSquare", #{ num: N(num) });

    let lo = 1;
    let hi = num;
    set(#{ lo: N(lo), hi: N(hi) });
    step("bounds", `If ${num} is a square, its root is somewhere in 1..${num}.`,
        [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);

    let ok = false;
    let root = 0;
    while lo <= hi {
        step("loop", `Roots ${lo}..${hi} are still possible.`,
            [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);

        let mid = lo + (hi - lo) / 2;
        set(#{ mid: N(mid) });
        step("mid", `Try ${mid} as the root.`,
            [ text("search range", [`lo = ${lo}`, `mid = ${mid}`, `hi = ${hi}`]) ]);

        let sq = mid * mid;
        step("square", `${mid} × ${mid} = ${sq}.`,
            [ text("test", [`${mid}² = ${sq}`, `num = ${num}`]) ]);

        if sq == num {
            ok = true;
            root = mid;
            log(`${num} = ${mid}²`, "result");
            step("hit", `${sq} is exactly ${num}.`,
                [ text("test", [`${mid}² = ${sq} = ${num}`, "exact hit"]) ]);
            step("retTrue", `${num} is the square of ${mid}.`,
                [ text("answer", [`${num} = ${mid}²`]) ], "return");
            break;
        }

        if sq < num {
            step("small", `${sq} < ${num}, so no root at or below ${mid} can work.`,
                [ text("test", [`${mid}² = ${sq} < ${num}`, "root must be larger"]) ]);
            lo = mid + 1;
            set(#{ lo: N(lo) });
            step("up", `lo = ${lo}.`,
                [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);
        } else {
            step("big", `${sq} > ${num}, so no root at or above ${mid} can work.`,
                [ text("test", [`${mid}² = ${sq} > ${num}`, "root must be smaller"]) ]);
            hi = mid - 1;
            set(#{ hi: N(hi) });
            step("down", `hi = ${hi}.`,
                [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);
        }
    }

    if !ok {
        log(`${num} is not a perfect square`, "result");
        step("retFalse", `The range emptied without an exact hit: ${num} lies strictly between two consecutive squares.`,
            [ text("answer", [`${num} is not a perfect square`]) ], "return");
    }

    result(`${ok}`);
    leave(`${ok}`);
}
