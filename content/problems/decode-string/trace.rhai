// Decode String — the stacks hold what is *suspended*, not what is finished.
//
// Nesting is the whole difficulty: when "[" arrives, whatever text was being
// assembled is not wrong, it is merely interrupted. Pushing it aside (with the
// repeat count that will govern the interruption) and starting fresh means
// that at "]" the current text is always exactly the innermost group — ready
// to be multiplied and glued back onto what was waiting underneath.

fn trace(input) {
    let s = input.s;
    let cs = chars(s);
    let counts = [];
    let parts = [];
    let cur = "";
    let n = 0;

    enter("decodeString", #{});
    step("init", "Repeat counts waiting for their closing bracket.",
        [ array("input", cs), stack("counts", counts) ]);
    step("parts", "Text that was interrupted by an opening bracket.",
        [ array("input", cs), stack("suspended text", parts) ]);
    set(#{ cur: S(cur) });
    step("cur", "The group currently being assembled.",
        [ array("input", cs), text("current", [`"${cur}"`]) ]);
    set(#{ n: N(n) });
    step("num", "A repeat count being read digit by digit.",
        [ array("input", cs), text("current", [`"${cur}"`]) ]);

    for i in 0..cs.len() {
        let ch = cs[i];
        step("loop", `Character ${i}: "${ch}".`,
            [ array("input", cs).ptr("i", i).done(before(i)),
              stack("counts", counts), stack("suspended text", parts) ]);

        if ord(ch) >= 48 && ord(ch) <= 57 {
            step("isDigit", `"${ch}" is a digit, so it is part of a repeat count.`,
                [ array("input", cs).ptr("i", i).hl([i]) ]);
            n = n * 10 + num(ch);
            set(#{ n: N(n) });
            step("accum", `Count so far: ${n}. Reading digit by digit is what allows counts above 9.`,
                [ array("input", cs).ptr("i", i), text("current", [`count = ${n}`, `text = "${cur}"`]) ]);
        } else if ch == "[" {
            step("isOpen", `"[" opens a group that must be repeated ${n} times.`,
                [ array("input", cs).ptr("i", i).hl([i]) ]);
            counts.push(n);
            step("pushCount", `Set ${n} aside until the matching "]".`,
                [ array("input", cs).ptr("i", i), stack("counts", counts).pushed(true) ]);
            parts.push(cur);
            step("pushStr", `Suspend "${cur}" — it is not finished, just interrupted.`,
                [ array("input", cs).ptr("i", i), stack("suspended text", parts).pushed(true) ]);
            n = 0;
            cur = "";
            set(#{ n: N(n), cur: S(cur) });
            step("reset", "Start the inner group from nothing.",
                [ array("input", cs).ptr("i", i), text("current", [`count = 0`, `text = ""`]) ]);
        } else if ch == "]" {
            step("isClose", `"]" closes the innermost group, and "${cur}" is exactly its contents.`,
                [ array("input", cs).ptr("i", i).hl([i]), text("current", [`"${cur}"`]) ]);
            let k = counts[counts.len() - 1];
            counts.truncate(counts.len() - 1);
            step("popCount", `That group repeats ${k} times.`,
                [ array("input", cs).ptr("i", i), stack("counts", counts).popped(k) ]);
            let prev = parts[parts.len() - 1];
            parts.truncate(parts.len() - 1);
            step("popStr", `The text it interrupted was "${prev}".`,
                [ array("input", cs).ptr("i", i), stack("suspended text", parts).popped(prev) ]);
            cur = prev + repeat(cur, k);
            set(#{ cur: S(cur) });
            log(`expand to "${cur}"`);
            step("expand", `Multiply out and glue back on: "${cur}".`,
                [ array("input", cs).ptr("i", i), text("current", [`"${cur}"`]) ]);
        } else {
            step("isLetter", `"${ch}" is ordinary text.`,
                [ array("input", cs).ptr("i", i).hl([i]) ]);
            cur += ch;
            set(#{ cur: S(cur) });
            step("append", `Append it: "${cur}".`,
                [ array("input", cs).ptr("i", i), text("current", [`"${cur}"`]) ]);
        }
    }

    log(`answer ${cur}`, "result");
    result(cur);
    step("ret", `Both stacks are empty, so nothing is suspended and "${cur}" is the whole decoding.`,
        [ array("input", cs).done(seq(0, cs.len() - 1)), text("current", [`"${cur}"`]) ], "return");
    leave(cur);
}
