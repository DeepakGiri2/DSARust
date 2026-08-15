// Reverse Integer — test for overflow before causing it.
//
// The digit shuffling is routine; the guard is the point. `out * 10 + d` must
// stay within 2147483647, so the test is whether `out` already exceeds
// (2147483647 − d) / 10. That form is deliberate: every value in it is small
// enough to compute safely, whereas checking the product afterwards would mean
// the overflow had already happened — and in a fixed-width type there is no
// way to detect it after the fact.
//
// The magnitude is reversed with the sign stripped off, so the same limit
// applies whichever way round the number was.

fn trace(input) {
    let x = input.x;
    let original = x;

    enter("reverse", #{ x: N(x) });

    let sign = 1;
    if x < 0 {
        sign = -1;
        x = -x;
    }
    set(#{ sign: N(sign) });
    step("sign",
        if sign < 0 { `Negative, so work with ${x} and put the sign back at the end.` }
        else { `Not negative, so the sign is simply 1.`.to_string() },
        [ text("state", [`sign = ${sign}`, `magnitude = ${x}`]) ]);

    let out = 0;
    set(#{ out: N(out) });
    step("init", "Build the reversed magnitude here.",
        [ text("state", [`remaining = ${x}`, `reversed = ${out}`]) ]);

    let overflowed = false;
    while x > 0 {
        step("loop", `${x} still has digits.`,
            [ text("state", [`remaining = ${x}`, `reversed = ${out}`]) ]);

        let d = x % 10;
        step("digit", `The last digit is ${d}.`,
            [ text("state", [`digit = ${d}`, `reversed = ${out}`]) ]);

        let limit = (2147483647 - d) / 10;
        if out > limit {
            overflowed = true;
            log("would overflow 32 bits", "result");
            step("overflow", `${out} × 10 + ${d} would exceed 2147483647. The test compares ${out} against (2147483647 − ${d}) / 10 = ${limit} — every value there is safe to compute, whereas checking the product afterwards would be too late.`,
                [ text("overflow check", [`reversed = ${out}`, `limit = ${limit}`,
                                          "would overflow"]) ]);
            step("fail", "So return 0.", [ text("answer", ["0"]) ], "return");
            break;
        }
        step("overflow", `${out} is within the limit of ${limit}, so the next step is safe.`,
            [ text("overflow check", [`reversed = ${out}`, `limit = ${limit}`, "safe"]) ]);

        out = out * 10 + d;
        set(#{ out: N(out) });
        step("push", `Push ${d} on: ${out}.`,
            [ text("state", [`reversed = ${out}`]) ]);

        x /= 10;
        set(#{ x: N(x) });
        step("drop", `Remaining: ${x}.`,
            [ text("state", [`remaining = ${x}`, `reversed = ${out}`]) ]);
    }

    if overflowed {
        result("0");
        leave("0");
        return;
    }

    let answer = sign * out;
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `${original} reversed is ${answer}.`,
        [ text("answer", [`${answer}`]) ], "return");
    leave(`${answer}`);
}
