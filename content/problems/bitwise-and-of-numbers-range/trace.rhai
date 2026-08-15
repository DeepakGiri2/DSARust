// Bitwise AND of a Range — the answer is the common prefix, nothing else.
//
// A bit survives the AND only if every number in the range has it set. If two
// numbers in the range differ in some bit, then somewhere between them that
// bit was 0, so it cannot survive. What is left is exactly the prefix the two
// endpoints share. Shifting both right until they are equal strips away every
// position where they could differ; shifting back restores the prefix in
// place, with zeros below it.

fn validate(input) {
    if input.left > input.right {
        return "left must not be greater than right";
    }
}

fn trace(input) {
    let left = input.left;
    let right = input.right;
    let l0 = left;
    let r0 = right;

    enter("rangeBitwiseAnd", #{ left: N(left), right: N(right) });

    let shift = 0;
    set(#{ shift: N(shift) });
    step("init", `The AND of every number from ${left} to ${right}. Nothing will be enumerated.`,
        [ bits("endpoints", [["left", left, 8], ["right", right, 8]]),
          text("state", [`shift = ${shift}`]) ]);

    while left < right {
        step("loop", `${left} and ${right} still differ, so somewhere in between the lowest differing bit must have been 0 — it cannot survive the AND.`,
            [ bits("endpoints", [["left", left, 8], ["right", right, 8]]),
              text("state", [`shift = ${shift}`]) ]);

        left = left >> 1;
        step("shiftLeft", `Drop the lowest bit of the left end: ${left}.`,
            [ bits("endpoints", [["left", left, 8], ["right", right, 8]]) ]);

        right = right >> 1;
        step("shiftRight", `And of the right end: ${right}.`,
            [ bits("endpoints", [["left", left, 8], ["right", right, 8]]) ]);

        shift += 1;
        set(#{ shift: N(shift) });
        log(`shifted ${shift} time(s); ${left} vs ${right}`);
        step("count", `${shift} position(s) discarded so far.`,
            [ text("state", [`shift = ${shift}`]) ]);
    }

    let answer = left << shift;
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `The endpoints agree on ${left}, which is their common prefix. Shift it back ${shift} place(s) to get ${answer}; every bit below it is 0 because it varied somewhere in the range.`,
        [ bits("result", [["common prefix", left, 8], ["shifted back", answer, 8]]),
          text("answer", [`AND of ${l0}..${r0} = ${answer}`]) ], "return");
    leave(`${answer}`);
}
