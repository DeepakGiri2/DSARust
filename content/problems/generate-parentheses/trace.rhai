// Generate Parentheses — two counters make every string valid by construction.
//
// There is no "is this balanced?" check anywhere. An open is allowed while
// fewer than n have been used; a close only while it would not outnumber the
// opens. That is the definition of well-formed, applied one character at a
// time, so no invalid string is ever built in the first place.

fn back(n, cur, open, close, found) {
    enter(`back("${cur}", ${open}, ${close})`, #{ open: N(open), close: N(close) });

    if cur.len() == 2 * n {
        found.push(cur);
        log(`found ${cur}`, "result");
        step("base", `"${cur}" is ${2 * n} characters long, so it is complete.`,
            [ array("current", chars(cur)).hl(seq(0, cur.len() - 1)) ]);
        step("add", `Record it — combination ${found.len()}.`, [ text("combinations", found) ]);
        step("ret", "Unwind and try a different choice.", [ text("combinations", found) ], "return");
        leave("");
        return found;
    }

    step("base", `"${cur}" is ${cur.len()} of ${2 * n} characters.`,
        [ array("current", chars(cur)),
          text("counters", [`opens used: ${open} of ${n}`, `closes used: ${close}`]) ]);

    if open < n {
        step("canOpen", `Only ${open} of ${n} opens are used, so another is allowed.`,
            [ array("current", chars(cur + "(")).hl([cur.len()]) ]);
        step("recOpen", `Try "${cur}(".`, [ array("current", chars(cur + "(")) ], "call");
        found = back(n, cur + "(", open + 1, close, found);
    } else {
        step("canOpen", `All ${n} opens are used — no more may be added.`,
            [ array("current", chars(cur)) ]);
    }

    if close < open {
        step("canClose", `${close} closes against ${open} opens, so a close cannot go negative.`,
            [ array("current", chars(cur + ")")).hl([cur.len()]) ]);
        step("recClose", `Try "${cur})".`, [ array("current", chars(cur + ")")) ], "call");
        found = back(n, cur + ")", open, close + 1, found);
    } else {
        step("canClose", `Closing now would give ${close + 1} closes against ${open} opens — unbalanceable, so this branch is never entered.`,
            [ array("current", chars(cur)) ]);
    }

    leave("");
    found
}

fn trace(input) {
    let n = input.n;

    enter("generateParenthesis", #{ n: N(n) });
    step("init", `Every valid arrangement of ${n} pair(s).`, [ text("combinations", []) ]);
    step("call", "Start from the empty string with both counters at zero.", [], "call");

    let found = back(n, "", 0, 0, []);

    let answer = join(found, " ");
    result(answer);
    step("done", `${plural(found.len(), "arrangement")}, none of which needed validating: ${answer}.`,
        [ text("combinations", found) ], "return");
    leave(answer);
}
