// Hamming Distance — XOR marks the disagreements, then count the marks.
//
// The counting trick is worth understanding rather than memorising. In binary,
// subtracting 1 turns the lowest set bit into 0 and every bit below it into 1;
// ANDing with the original therefore keeps every higher bit unchanged, clears
// that lowest set bit, and zeroes the ones below it that were already zero. So
// each iteration removes exactly one 1, and the loop runs once per set bit
// rather than once per bit position.

fn trace(input) {
    let x = input.x;
    let y = input.y;

    enter("hammingDistance", #{ x: N(x), y: N(y) });

    let diff = x ^ y;
    step("xor", `${x} ^ ${y} = ${diff}: a 1 in every column where the two disagree, and 0 where they match.`,
        [ bits("bits", [["x", x, 8], ["y", y, 8], ["x ^ y", diff, 8]]) ]);

    let count = 0;
    set(#{ count: N(count) });
    step("init", "Now just count the 1s.",
        [ bits("bits", [["x ^ y", diff, 8]]), text("count", [`${count}`]) ]);

    while diff > 0 {
        step("loop", `${diff} still has set bits.`,
            [ bits("bits", [["diff", diff, 8], ["diff - 1", diff - 1, 8]]),
              text("count", [`${count}`]) ]);

        let before = diff;
        diff = diff & (diff - 1);
        step("clear", `${before} - 1 flips the lowest 1 to 0 and everything below it to 1, so ANDing removes exactly that one bit: ${diff}.`,
            [ bits("bits", [["before", before, 8], ["before - 1", before - 1, 8],
                            ["after AND", diff, 8]]) ]);

        count += 1;
        set(#{ count: N(count) });
        log(`cleared one bit; count ${count}`);
        step("bump", `That is ${plural(count, "differing position")}.`,
            [ bits("bits", [["diff", diff, 8]]), text("count", [`${count}`]) ]);
    }

    log(`answer ${count}`, "result");
    result(`${count}`);
    step("ret", `${x} and ${y} differ in ${plural(count, "position")}. The loop ran ${count} time(s), not once per bit.`,
        [ bits("bits", [["x", x, 8], ["y", y, 8], ["x ^ y", x ^ y, 8]]),
          text("count", [`${count}`]) ], "return");
    leave(`${count}`);
}
