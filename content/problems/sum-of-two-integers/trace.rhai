// Sum of Two Integers — XOR is addition, AND finds where it went wrong.
//
// Column by column, 0+0, 0+1 and 1+0 are exactly XOR. Only 1+1 differs: it
// produces 0 with a carry, and AND is precisely the "both were 1" test.
// Shifting that left by one puts each carry into the column it belongs in, and
// the process repeats. It terminates because every round moves the carries at
// least one bit further left, so they run out of room.

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

fn trace(input) {
    let a = input.a;
    let b = input.b;

    enter("getSum", #{ a: N(a), b: N(b) });

    let sum = a;
    let carry = b;
    set(#{ sum: N(sum), carry: N(carry) });
    step("init", `Start with sum = ${a} and carry = ${b}. From here "carry" means "still to be added in".`,
        [ bits("bits", [["sum", sum, 10], ["carry", carry, 10]]),
          text("state", [`sum = ${sum} (${bits_of(sum)})`, `carry = ${carry} (${bits_of(carry)})`]) ]);

    let round = 0;
    while carry != 0 {
        round += 1;
        step("loop", `Round ${round}: there are still carries to fold in.`,
            [ bits("bits", [["sum", sum, 10], ["carry", carry, 10]]),
              text("state", [`sum = ${sum}`, `carry = ${carry}`]) ]);

        let next = (sum & carry) << 1;
        step("carry", `Columns where both are 1: ${sum} & ${carry} = ${sum & carry}. Those are exactly the 1+1 cases, and each one carries into the column to its left — so shift by one to get ${next}.`,
            [ bits("carries", [["sum & carry", sum & carry, 10], ["shifted left", next, 10]]),
              text("state", [`new carry = ${next}`]) ]);

        sum = sum ^ carry;
        set(#{ sum: N(sum) });
        step("add", `XOR adds every column while ignoring carries: ${sum}.`,
            [ bits("bits", [["sum", sum, 10]]),
              text("state", [`sum = ${sum} (${bits_of(sum)})`]) ]);

        carry = next;
        set(#{ carry: N(carry) });
        log(`round ${round}: sum ${sum}, carry ${carry}`);
        step("shift", `Carry becomes ${carry}. It has moved left, which is why this cannot go on forever.`,
            [ bits("bits", [["sum", sum, 10], ["carry", carry, 10]]),
              text("state", [`sum = ${sum}`, `carry = ${carry}`]) ]);
    }

    log(`answer ${sum}`, "result");
    result(`${sum}`);
    step("ret", `No carries left, so ${a} + ${b} = ${sum}.`,
        [ bits("bits", [["sum", sum, 10]]), text("answer", [`${a} + ${b} = ${sum}`]) ], "return");
    leave(`${sum}`);
}
