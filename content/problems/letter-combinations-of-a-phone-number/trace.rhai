// Letter Combinations — one level of recursion per digit.
//
// The shape is a tree whose depth is the number of digits and whose branching
// factor is that digit's letter count. Every leaf is one complete word, so
// nothing needs filtering afterwards.

fn letters_for(d) {
    if d == "2" { "abc" } else if d == "3" { "def" } else if d == "4" { "ghi" }
    else if d == "5" { "jkl" } else if d == "6" { "mno" } else if d == "7" { "pqrs" }
    else if d == "8" { "tuv" } else { "wxyz" }
}

fn back(digits, i, cur, found) {
    enter(`back(${i}, "${cur}")`, #{ depth: N(i) });

    if i == digits.len() {
        found.push(cur);
        log(`found ${cur}`, "result");
        step("base", `All ${digits.len()} digit(s) have a letter.`,
            [ array("digits", digits).done(seq(0, digits.len() - 1)),
              array("current", chars(cur)).hl(seq(0, cur.len() - 1)) ]);
        step("add", `Record "${cur}".`, [ text("combinations", found) ]);
        step("ret", "Unwind.", [ text("combinations", found) ], "return");
        leave("");
        return found;
    }

    let opts = letters_for(digits[i]);
    step("base", `Digit ${digits[i]} offers "${opts}".`,
        [ array("digits", digits).ptr("i", i).hl([i]),
          array("current", chars(cur)) ]);

    for ch in chars(opts) {
        step("loop", `Try '${ch}' for digit ${digits[i]}.`,
            [ array("digits", digits).ptr("i", i),
              array("current", chars(cur + ch)).hl([cur.len()]) ]);
        step("rec", `Move to digit ${i + 1}.`,
            [ array("current", chars(cur + ch)) ], "call");
        found = back(digits, i + 1, cur + ch, found);
    }

    leave("");
    found
}

fn trace(input) {
    let digits = chars(input.digits);

    enter("letterCombinations", #{ digits: S(input.digits) });
    if digits.len() == 0 {
        step("empty", "No digits.", [ text("combinations", []) ]);
        result("0");
        step("retEmpty", "Return nothing.", [ text("combinations", []) ], "return");
        leave("0");
        return;
    }
    step("empty", `${digits.len()} digit(s) to spell.`, [ array("digits", digits) ]);
    step("init", "Collect every word.", [ array("digits", digits) ]);
    step("call", "Start at digit 0.", [ array("digits", digits).ptr("i", 0) ], "call");

    let found = back(digits, 0, "", []);

    result(`${found.len()}`);
    step("done", `${found.len()} combination(s): ${join(found, " ")}.`,
        [ text("combinations", found) ], "return");
    leave(`${found.len()}`);
}
