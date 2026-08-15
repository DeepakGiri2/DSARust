// Reverse Bits — read from the bottom, write to the top.
//
// Each round makes room in the result by shifting it left, drops in the
// input's lowest bit, then discards that bit. The first bit read gets shifted
// left once per remaining round, so it ends up at the top — which is exactly
// what reversal means.

fn trace(input) {
    let n = input.n;
    let width = 8;
    let res = 0;

    enter("reverseBits", #{ n: N(n) });
    set(#{ res: N(res) });
    step("init", `Reverse the ${width} bits of ${n}.`,
        [ bits("registers", [["n", n, width], ["res", res, width]]) ]);

    for i in 0..width {
        set(#{ i: N(i) });
        step("loop", `Round ${i + 1} of ${width}.`,
            [ bits("registers", [["n", n, width, [0]], ["res", res, width]]) ]);

        res = res << 1;
        set(#{ res: N(res) });
        step("shiftRes", "Shift the result left to open a slot at the bottom.",
            [ bits("registers", [["n", n, width, [0]], ["res", res, width, [0]]]) ]);

        let bit = n & 1;
        res = res | bit;
        set(#{ res: N(res) });
        log(`take bit ${bit}`);
        step("take", `The lowest bit of n is ${bit}; drop it into the slot.`,
            [ bits("registers", [["n", n, width, [0]], ["res", res, width, [0]]]) ]);

        n = n >> 1;
        set(#{ n: N(n) });
        step("shiftN", "Discard it from n.",
            [ bits("registers", [["n", n, width], ["res", res, width]]) ]);
    }

    log(`answer ${res}`, "result");
    result(`${res}`);
    step("ret", `The first bit read ended up at the top: ${res}.`,
        [ bits("registers", [["res", res, width]]) ], "return");
    leave(`${res}`);
}
