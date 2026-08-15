// Excel Column Title — base 26 with no zero digit.
//
// In real base 26 the digits are 0..25 and 26 would be written "10". Here the
// digits are A..Z, meaning 1..26, and there is nothing to put in the tens
// place when the units digit is 26. The single `columnNumber--` before each
// step shifts the digit back into the 0..25 range that ordinary division and
// remainder expect, and shifts the borrow into the quotient at the same time.

fn alpha(r) {
    chr(65 + r)
}

fn trace(input) {
    let n = input.columnNumber;
    let original = n;
    let out = "";

    enter("convertToTitle", #{ columnNumber: N(n) });
    step("init", `Peel one letter at a time off ${n}, least significant first.`,
        [ text("state", [`remaining: ${n}`, `title so far: ""`]) ]);

    while n > 0 {
        step("loop", `${n} still has letters in it.`,
            [ text("state", [`remaining: ${n}`, `title so far: "${out}"`]) ]);

        let before = n;
        n -= 1;
        step("shift", `Subtract 1 first: ${before} becomes ${n}. A is 1 rather than 0, so without this the remainder would run 1..26 and never fit a 0-based digit.`,
            [ text("state", [`${before} - 1 = ${n}`]) ]);

        let r = n % 26;
        step("digit", `${n} mod 26 = ${r}, which is the letter ${alpha(r)}.`,
            [ text("state", [`digit: ${r} -> ${alpha(r)}`]) ]);

        out = alpha(r) + out;
        log(`letter ${alpha(r)}`);
        step("prepend", `Letters come out backwards, so put it in front: "${out}".`,
            [ text("state", [`title so far: "${out}"`]) ]);

        n /= 26;
        step("divide", `Divide by 26 to move to the next place: ${n} remains.`,
            [ text("state", [`remaining: ${n}`, `title so far: "${out}"`]) ]);
    }

    log(`answer ${out}`, "result");
    result(out);
    step("ret", `Column ${original} is "${out}".`, [ text("answer", [out]) ], "return");
    leave(out);
}
