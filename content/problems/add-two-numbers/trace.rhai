// Add Two Numbers — the digits are already in the direction addition wants.
//
// Reverse order means the head is the ones digit, so walking forwards is
// walking from least significant to most — exactly how carries propagate. A
// list that has run out simply contributes 0, which removes the need to pad
// the shorter number.

fn trace(input) {
    let a = input.l1;
    let b = input.l2;
    let out = [];
    let carry = 0;
    let i = 0;
    let j = 0;

    enter("addTwoNumbers", #{});
    step("dummy", "A dummy head so appending never needs a special first case.",
        [ array("l1 (ones digit first)", a), array("l2 (ones digit first)", b) ]);
    set(#{ carry: N(carry) });
    step("init", "No carry yet.",
        [ array("l1 (ones digit first)", a).ptr("i", i),
          array("l2 (ones digit first)", b).ptr("j", j) ]);

    while i < a.len() || j < b.len() || carry != 0 {
        step("loop", "Something is still left to add.",
            [ array("l1 (ones digit first)", a).ptr("i", i).done(before(i)),
              array("l2 (ones digit first)", b).ptr("j", j).done(before(j)),
              array("sum", out) ]);

        let sum = carry;
        set(#{ sum: N(sum) });
        step("sum", `Start from the carry: ${sum}.`,
            [ array("sum", out) ]);

        if i < a.len() {
            sum += a[i];
            step("takeA", `Add l1's digit ${a[i]}: ${sum}.`,
                [ array("l1 (ones digit first)", a).ptr("i", i).hl([i]) ]);
            i += 1;
        } else {
            step("takeA", "l1 is exhausted, so it contributes 0 — no padding needed.",
                [ array("l1 (ones digit first)", a).done(seq(0, a.len() - 1)) ]);
        }

        if j < b.len() {
            sum += b[j];
            step("takeB", `Add l2's digit ${b[j]}: ${sum}.`,
                [ array("l2 (ones digit first)", b).ptr("j", j).hl([j]) ]);
            j += 1;
        } else {
            step("takeB", "l2 is exhausted, so it contributes 0.",
                [ array("l2 (ones digit first)", b).done(seq(0, b.len() - 1)) ]);
        }

        carry = sum / 10;
        set(#{ carry: N(carry) });
        step("carry", `${sum} carries ${carry} into the next column.`, [ array("sum", out) ]);

        out.push(sum % 10);
        log(`digit ${sum % 10}, carry ${carry}`);
        step("digit", `The digit here is ${sum} % 10 = ${sum % 10}.`,
            [ array("sum", out).hl([out.len() - 1]) ]);
        step("advance", "Move to the next column.", [ array("sum", out) ]);
    }

    let answer = join(out, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `The sum, also ones digit first: ${answer}.`,
        [ array("sum", out).hl(seq(0, out.len() - 1)) ], "return");
    leave(answer);
}
