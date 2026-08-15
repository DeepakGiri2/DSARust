// Single Number II — a counter modulo 3, run on every bit at once.
//
// For each bit position, count how often it has been 1 and keep only the
// remainder modulo three. Three states need two bits, so two whole words hold
// all the counters in parallel: `ones` marks the positions currently at
// remainder 1, `twos` the positions at remainder 2, and a position at 0 is in
// neither.
//
// The masks are what implement the wrap-around. A bit reaching two is removed
// from `ones` by `& ~twos`; a bit reaching three is then removed from `twos`
// by `& ~ones` — using the *just updated* `ones`, which is why the order of
// the two assignments matters.

// Rhai has no integer NOT operator, and ~b == b ^ -1 in two's complement.
fn bnot(b) { b ^ -1 }

fn state_lines(ones, twos) {
    [`ones (seen 1 mod 3): ${ones}`, `twos (seen 2 mod 3): ${twos}`]
}

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("singleNumber", #{ n: N(n) });

    let ones = 0;
    let twos = 0;
    set(#{ ones: N(ones), twos: N(twos) });
    step("init", "Every bit position starts at remainder 0 — in neither word.",
        [ array("nums", nums), bits("counters", [["ones", ones, 8], ["twos", twos, 8]]),
          text("state", state_lines(ones, twos)) ]);

    for i in 0..n {
        let v = nums[i];
        step("loop", `Fold in ${v}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              bits("counters", [["value", v, 8], ["ones", ones, 8], ["twos", twos, 8]]),
              text("state", state_lines(ones, twos)) ]);

        let seen_once = ones ^ v;
        step("xorOnes", `${ones} ^ ${v} = ${seen_once}: every bit of ${v} flips its membership of "seen once".`,
            [ bits("step", [["ones", ones, 8], ["value", v, 8], ["XOR", seen_once, 8]]) ]);

        ones = seen_once & bnot(twos);
        set(#{ ones: N(ones) });
        step("maskOnes", `Mask out anything already at remainder 2: ones = ${ones}. A bit that has now been seen twice belongs in twos, not here.`,
            [ bits("step", [["XOR", seen_once, 8], ["twos", twos, 8], ["ones", ones, 8]]),
              text("state", state_lines(ones, twos)) ]);

        let seen_twice = twos ^ v;
        step("xorTwos", `${twos} ^ ${v} = ${seen_twice}: the same flip for "seen twice".`,
            [ bits("step", [["twos", twos, 8], ["value", v, 8], ["XOR", seen_twice, 8]]) ]);

        twos = seen_twice & bnot(ones);
        set(#{ twos: N(twos) });
        log(`after ${v}: ones ${ones}, twos ${twos}`);
        step("maskTwos", `Mask with the *new* ones: twos = ${twos}. A bit that has now been seen three times is cleared from both words and is back at remainder 0.`,
            [ bits("counters", [["ones", ones, 8], ["twos", twos, 8]]),
              text("state", state_lines(ones, twos)) ]);
    }

    log(`answer ${ones}`, "result");
    result(`${ones}`);
    step("ret", `Every value appearing three times has cycled back to remainder 0, so what remains in ones is the loner: ${ones}.`,
        [ bits("counters", [["ones", ones, 8], ["twos", twos, 8]]),
          text("answer", [`${ones}`]) ], "return");
    leave(`${ones}`);
}
