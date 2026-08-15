// Decode Ways — Fibonacci, but each addition has to earn its place.
//
// A decoding is built by consuming either one digit or two, so the number of
// decodings of the first i characters is the number one back (if this digit
// stands alone) plus the number two back (if this digit pairs with the one
// before it). The two conditions are the whole problem: '0' cannot be a letter
// by itself, and a pair is only a letter if it reads 10..26 — which is why
// "06" is not a valid pair even though 6 is a valid letter.

fn validate(input) {
    if input.s.len() == 0 {
        return "the string must not be empty";
    }
    for c in chars(input.s) {
        if ord(c) < 48 || ord(c) > 57 {
            return `"${c}" is not a digit`;
        }
    }
}

fn trace(input) {
    let s = input.s;
    let cs = chars(s);
    let n = cs.len();

    enter("numDecodings", #{});

    if cs[0] == "0" {
        log("leading zero", "result");
        step("leadZero", "The string starts with 0, and no letter is encoded as 0 — so there is no valid decoding at all.",
            [ array("digits", cs).bad([0]) ], "return");
        result("0");
        leave("0");
        return;
    }
    step("leadZero", `It starts with ${cs[0]}, which is a letter on its own, so at least one decoding exists.`,
        [ array("digits", cs).hl([0]) ]);

    let prev2 = 1;
    let prev1 = 1;
    let counts = [1];
    set(#{ prev2: N(prev2), prev1: N(prev1) });
    step("init", "One way to decode the empty prefix, and one way to decode the first digit.",
        [ array("digits", cs).done([0]),
          text("state", [`ways(0 digits) = ${prev2}`, `ways(1 digit) = ${prev1}`]) ]);

    for i in 1..n {
        step("loop", `Extending to digit ${i}, "${cs[i]}".`,
            [ array("digits", cs).ptr("i", i).done(before(i)),
              text("state", [`ways two back = ${prev2}`, `ways one back = ${prev1}`]) ]);

        let cur = 0;
        step("cur", "Count the ways any decoding could end here.",
            [ array("digits", cs).ptr("i", i).hl([i]), text("state", [`cur = ${cur}`]) ]);

        if cs[i] != "0" {
            step("single", `"${cs[i]}" is not 0, so it can be a letter on its own.`,
                [ array("digits", cs).ptr("i", i).hl([i]) ]);
            cur += prev1;
            step("addSingle", `Every decoding of the first ${i} digits extends by that one letter: + ${prev1} = ${cur}.`,
                [ array("digits", cs).ptr("i", i).hl([i]), text("state", [`cur = ${cur}`]) ]);
        } else {
            step("single", `"${cs[i]}" is 0, which is not a letter — so no decoding can end with it alone.`,
                [ array("digits", cs).ptr("i", i).bad([i]) ]);
        }

        let two = num(cs[i - 1]) * 10 + num(cs[i]);
        step("pair", `Together with the digit before it, "${cs[i - 1]}${cs[i]}" reads as ${two}.`,
            [ array("digits", cs).ptr("i", i).hl([i - 1, i]) ]);

        if two >= 10 && two <= 26 {
            step("valid", `${two} is between 10 and 26, so the pair is a single letter. (Below 10 would mean a leading zero; above 26 is past Z.)`,
                [ array("digits", cs).ptr("i", i).hl([i - 1, i]) ]);
            cur += prev2;
            step("addPair", `Every decoding of the first ${i - 1} digits extends by that letter: + ${prev2} = ${cur}.`,
                [ array("digits", cs).ptr("i", i).hl([i - 1, i]), text("state", [`cur = ${cur}`]) ]);
        } else {
            step("valid", `${two} is outside 10..26, so those two digits cannot be one letter.`,
                [ array("digits", cs).ptr("i", i).bad([i - 1, i]) ]);
        }

        prev2 = prev1;
        prev1 = cur;
        counts.push(cur);
        set(#{ prev2: N(prev2), prev1: N(prev1) });
        log(`ways for the first ${i + 1} digits: ${cur}`);
        step("shift", `Slide the two-value window forward.`,
            [ array("ways per prefix length", counts).hl([counts.len() - 1]),
              text("state", [`ways two back = ${prev2}`, `ways one back = ${prev1}`]) ]);
    }

    log(`answer ${prev1}`, "result");
    result(`${prev1}`);
    step("ret", `${plural(prev1, "decoding")} of "${s}".`,
        [ array("ways per prefix length", counts).hl([counts.len() - 1]) ], "return");
    leave(`${prev1}`);
}
