// Add Binary — one loop condition removes three special cases.
//
// "While either string has digits left, or a carry is outstanding" handles
// unequal lengths, the final carry, and the ordinary case with the same body.
// The alternative — padding the shorter string, then checking for a leftover
// carry afterwards — is more code and more places to be wrong.

fn trace(input) {
    let a = chars(input.a);
    let b = chars(input.b);

    enter("addBinary", #{});

    let i = a.len() - 1;
    let j = b.len() - 1;
    set(#{ i: N(i), j: N(j) });
    step("init", "Start at the rightmost digit of each — the least significant end, where a carry originates.",
        [ array("a", a).ptr("i", i), array("b", b).ptr("j", j) ]);

    let carry = 0;
    set(#{ carry: N(carry) });
    step("carry", "No carry to begin with.", [ text("state", [`carry = ${carry}`]) ]);

    let out = [];
    step("out", "Digits are produced least significant first, so they come out backwards.",
        [ array("output (reversed)", out) ]);

    while i >= 0 || j >= 0 || carry > 0 {
        step("loop",
            if i >= 0 || j >= 0 {
                `Digits remain${if carry > 0 { ", and there is a carry" } else { "" }}.`
            } else {
                "Both strings are exhausted, but a carry is still outstanding — the same loop handles it.".to_string()
            },
            [ array("a", a).ptr("i", if i >= 0 { i } else { 0 }),
              array("b", b).ptr("j", if j >= 0 { j } else { 0 }),
              text("state", [`carry = ${carry}`]) ]);

        let sum = carry;
        step("start", `The column starts at the incoming carry, ${carry}.`,
            [ text("column", [`carry in: ${carry}`, `running: ${sum}`]) ]);

        if i >= 0 {
            sum += num(a[i]);
            step("takeA", `a contributes ${a[i]}: running total ${sum}.`,
                [ array("a", a).ptr("i", i).hl([i]), text("column", [`running: ${sum}`]) ]);
            i -= 1;
            set(#{ i: N(i) });
        } else {
            step("takeA", "a has run out, so it contributes nothing — no padding needed.",
                [ text("column", [`running: ${sum}`]) ]);
        }

        if j >= 0 {
            sum += num(b[j]);
            step("takeB", `b contributes ${b[j]}: running total ${sum}.`,
                [ array("b", b).ptr("j", j).hl([j]), text("column", [`running: ${sum}`]) ]);
            j -= 1;
            set(#{ j: N(j) });
        } else {
            step("takeB", "b has run out too.", [ text("column", [`running: ${sum}`]) ]);
        }

        out.push(`${sum % 2}`);
        step("bit", `${sum} in this column writes ${sum % 2}.`,
            [ array("output (reversed)", out).hl([out.len() - 1]) ]);

        carry = sum / 2;
        set(#{ carry: N(carry) });
        log(`column sum ${sum} -> bit ${sum % 2}, carry ${carry}`);
        step("nextCarry", `And carries ${carry} into the next column.`,
            [ text("state", [`carry = ${carry}`]) ]);
    }

    let rev = [];
    let k = out.len() - 1;
    while k >= 0 { rev.push(out[k]); k -= 1; }
    step("reverse", "Reverse to put the most significant digit first.",
        [ array("output (reversed)", out), array("result", rev) ]);

    let answer = join(rev, "");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${input.a} + ${input.b} = ${answer}.`,
        [ text("answer", [answer]) ], "return");
    leave(answer);
}
