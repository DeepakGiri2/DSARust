// Number of 1 Bits — n & (n-1) clears the lowest set bit.
//
// Subtracting 1 turns the lowest 1 into a 0 and every 0 below it into a 1.
// ANDing with the original therefore wipes exactly that one bit and leaves
// everything above untouched. The loop runs once per set bit, not once per
// bit, so a number with one bit set finishes in one step.

fn flipped_bits(before, after, width) {
    let out = [];
    for b in 0..width {
        if ((before >> b) & 1) != ((after >> b) & 1) { out.push(b); }
    }
    out
}

fn trace(input) {
    let n = input.n;
    let width = 8;
    let count = 0;

    enter("hammingWeight", #{ n: N(n) });
    set(#{ count: N(count) });
    step("init", `Count the 1 bits of ${n}.`,
        [ bits("n", [["n", n, width]]) ]);

    while n != 0 {
        step("loop", `${n} still has bits set.`,
            [ bits("n", [["n", n, width], ["n - 1", n - 1, width]]) ]);

        let before = n;
        n = n & (n - 1);
        let gone = flipped_bits(before, n, width);
        log(`${before} & ${before - 1} = ${n}`);
        step("clear",
            `${before} − 1 turns the lowest 1 into 0 and fills below it with 1s, so the AND wipes exactly bit ${gone[0]}.`,
            [ bits("n", [["n", n, width, gone], ["was", before, width, gone]]) ]);

        count += 1;
        set(#{ count: N(count) });
        step("count", `${count} bit(s) cleared so far.`,
            [ bits("n", [["n", n, width]]) ]);
    }

    log(`answer ${count}`, "result");
    result(`${count}`);
    step("ret", `n reached 0 after ${count} clearings, so it had ${count} set bit(s).`,
        [ bits("n", [["n", 0, width]]) ], "return");
    leave(`${count}`);
}
