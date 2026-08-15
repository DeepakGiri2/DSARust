// Pow(x, n) — read the exponent in binary.
//
// x^13 is x^8 · x^4 · x^1, because 13 is 1101 in binary. Squaring the base
// walks through x^1, x^2, x^4, x^8 at one multiplication each, and the bits of
// n say which of them to keep. Nothing about the exponent is recomputed — each
// halving throws away a bit that has already been used.

fn bits_of(n) {
    if n == 0 { return "0"; }
    let out = "";
    let k = n;
    while k > 0 {
        out = `${k % 2}` + out;
        k /= 2;
    }
    out
}

// Rhai always prints a decimal point on a float, so 1024.0 would read as
// "1024.0" where the answer is the whole number 1024.
fn fmt(v) {
    let s = `${v}`;
    if s.ends_with(".0") { s.sub_string(0, s.len() - 2) } else { s }
}

fn trace(input) {
    let x = input.x;
    let n = input.n;

    enter("myPow", #{ x: N(x), n: N(n) });

    if n < 0 {
        x = 1.0 / x;
        n = -n;
        step("negative", `A negative exponent is the same as a positive one over the reciprocal, so invert x to ${fmt(x)} and make the exponent ${n}. Handling it once here keeps the loop free of the case.`,
            [ text("setup", [`x = ${fmt(x)}`, `n = ${n}`, `n in binary = ${bits_of(n)}`]) ]);
    } else {
        step("negative", `The exponent is not negative, so x stays as it is. In binary ${n} is ${bits_of(n)}.`,
            [ text("setup", [`x = ${fmt(x)}`, `n = ${n}`, `n in binary = ${bits_of(n)}`]) ]);
    }

    let result = 1.0;
    set(#{ result: N(result) });
    step("init", "The product starts at 1 — the identity, so an exponent of 0 needs no special case.",
        [ text("state", [`result = ${fmt(result)}`]) ]);

    let base = x;
    set(#{ base: N(base) });
    step("base", `The base starts at x^1 = ${fmt(base)}. Each squaring will move it to the next power of two.`,
        [ text("state", [`result = ${fmt(result)}`, `base = ${fmt(base)}`, `n = ${n}`]) ]);

    let power = 1;
    while n > 0 {
        step("loop", `${n} still has bits left (${bits_of(n)}), and base currently holds x^${power}.`,
            [ bits("remaining exponent", [["n", n, 8]]),
              text("state", [`result = ${fmt(result)}`, `base = x^${power} = ${fmt(base)}`]) ]);

        if n % 2 == 1 {
            step("odd", `The lowest bit of ${n} is 1, so x^${power} is one of the pieces ${input.n} is made of.`,
                [ bits("remaining exponent", [["n", n, 8, [0]]]),
                  text("state", [`base = x^${power} = ${fmt(base)}`]) ]);
            result = result * base;
            set(#{ result: N(result) });
            log(`absorb x^${power}, result = ${fmt(result)}`);
            step("absorb", `Multiply it in: result = ${fmt(result)}.`,
                [ text("state", [`result = ${fmt(result)}`, `base = x^${power} = ${fmt(base)}`]) ]);
        } else {
            step("odd", `The lowest bit of ${n} is 0, so x^${power} is not part of the answer — skip it.`,
                [ bits("remaining exponent", [["n", n, 8, [0]]]),
                  text("state", [`result = ${fmt(result)}`]) ]);
        }

        base = base * base;
        power *= 2;
        set(#{ base: N(base) });
        step("square", `Square the base once: it now holds x^${power} = ${fmt(base)}. One multiplication bought a doubling of the exponent.`,
            [ text("state", [`result = ${fmt(result)}`, `base = x^${power} = ${fmt(base)}`]) ]);

        n /= 2;
        set(#{ n: N(n) });
        step("halve", `Drop the bit just used: ${n} remains (${bits_of(n)}).`,
            [ bits("remaining exponent", [["n", n, 8]]),
              text("state", [`result = ${fmt(result)}`, `n = ${n}`]) ]);
    }

    log(`answer ${fmt(result)}`, "result");
    result(fmt(result));
    step("ret", `Every bit of the exponent has been consumed: ${fmt(input.x)}^${input.n} = ${fmt(result)}.`,
        [ text("answer", [`${fmt(input.x)}^${input.n} = ${fmt(result)}`]) ], "return");
    leave(fmt(result));
}
