// Tribonacci — the window is as wide as the recurrence is deep.
//
// Nothing here is new relative to Fibonacci except the number 3. A recurrence
// that reaches back k places needs exactly k values kept alive; everything
// older can be discarded the moment the window slides past it.

fn trace(input) {
    let n = input.n;

    enter("tribonacci", #{ n: N(n) });

    if n == 0 {
        step("zero", "T(0) = 0 by definition.",
            [ text("definition", ["T(0) = 0", "T(1) = T(2) = 1", "T(k) = T(k-1) + T(k-2) + T(k-3)"]) ]);
        result("0");
        step("ret", "Return 0.", [ text("answer", ["T(0) = 0"]) ], "return");
        leave("0");
        return;
    }
    step("zero", `${n} is not 0.`,
        [ text("definition", ["T(0) = 0", "T(1) = T(2) = 1", "T(k) = T(k-1) + T(k-2) + T(k-3)"]) ]);

    if n < 3 {
        step("small", `T(${n}) = 1 by definition.`,
            [ text("definition", ["T(1) = 1", "T(2) = 1"]) ]);
        result("1");
        step("ret", "Return 1.", [ text("answer", [`T(${n}) = 1`]) ], "return");
        leave("1");
        return;
    }
    step("small", `${n} is past all three base cases, so the terms must be built up.`,
        [ text("definition", ["T(0) = 0", "T(1) = T(2) = 1"]) ]);

    let a = 0;
    let b = 1;
    let c = 1;
    let seq = [0, 1, 1];
    set(#{ a: N(a), b: N(b), c: N(c) });
    step("init", "Three base values, and therefore a window three wide.",
        [ array("terms computed", seq).hl([0, 1, 2]),
          text("window", [`T(0) = ${a}`, `T(1) = ${b}`, `T(2) = ${c}`]) ]);

    for i in 3..(n + 1) {
        step("loop", `Building T(${i}) from the three terms before it.`,
            [ array("terms computed", seq).hl([seq.len() - 3, seq.len() - 2, seq.len() - 1]),
              text("window", [`T(${i - 3}) = ${a}`, `T(${i - 2}) = ${b}`, `T(${i - 1}) = ${c}`]) ]);

        let next = a + b + c;
        log(`T(${i}) = ${a} + ${b} + ${c} = ${next}`);
        step("add", `${a} + ${b} + ${c} = ${next}.`,
            [ array("terms computed", seq).hl([seq.len() - 3, seq.len() - 2, seq.len() - 1]),
              text("window", [`T(${i}) = ${next}`]) ]);

        a = b;
        b = c;
        c = next;
        seq.push(c);
        set(#{ a: N(a), b: N(b), c: N(c) });
        step("shift", `Slide the window: T(${i - 3}) is now beyond reach and can be forgotten.`,
            [ bars("terms computed", seq).hl([seq.len() - 1]),
              text("window", [`T(${i - 2}) = ${a}`, `T(${i - 1}) = ${b}`, `T(${i}) = ${c}`]) ]);
    }

    log(`answer ${c}`, "result");
    result(`${c}`);
    step("ret", `T(${n}) = ${c}.`,
        [ bars("terms computed", seq).hl([seq.len() - 1]) ], "return");
    leave(`${c}`);
}
