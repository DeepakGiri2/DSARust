// Valid Parenthesis String — carry a range, not a decision.
//
// Committing to a meaning for each star means backtracking. Instead track how
// many brackets could still be open: `lo` assumes every star closed, `hi`
// assumes every star opened, and the truth is somewhere between. A negative
// `hi` means even the most generous reading has run out of openers, so the
// string is finished. A negative `lo` is not fatal — it just means the
// pessimistic assumption went too far, so it is clamped back to zero.

fn trace(input) {
    let s = input.s;
    let cs = chars(s);
    let n = cs.len();

    enter("checkValidString", #{});

    let lo = 0;
    let hi = 0;
    set(#{ lo: N(lo), hi: N(hi) });
    step("init", "lo: open brackets if every star closes. hi: if every star opens. The real count is always somewhere in between.",
        [ array("string", cs), text("possible open counts", [`from ${lo} to ${hi}`]) ]);

    let ok = true;
    for i in 0..n {
        let c = cs[i];
        step("loop", `Character ${i}: "${c}".`,
            [ array("string", cs).ptr("i", i).done(before(i)),
              text("possible open counts", [`from ${lo} to ${hi}`]) ]);

        if c == "(" {
            lo += 1;
            hi += 1;
            set(#{ lo: N(lo), hi: N(hi) });
            step("open", `An opener, so both bounds rise: ${lo}..${hi}.`,
                [ array("string", cs).ptr("i", i).hl([i]),
                  text("possible open counts", [`from ${lo} to ${hi}`]) ]);
        } else if c == ")" {
            lo -= 1;
            hi -= 1;
            set(#{ lo: N(lo), hi: N(hi) });
            step("close", `A closer, so both bounds fall: ${lo}..${hi}.`,
                [ array("string", cs).ptr("i", i).hl([i]),
                  text("possible open counts", [`from ${lo} to ${hi}`]) ]);
        } else {
            lo -= 1;
            hi += 1;
            set(#{ lo: N(lo), hi: N(hi) });
            step("star", `A star widens the range in both directions at once: ${lo}..${hi}. That is the whole point of not deciding what it means.`,
                [ array("string", cs).ptr("i", i).hl([i]),
                  text("possible open counts", [`from ${lo} to ${hi}`]) ]);
        }

        if hi < 0 {
            ok = false;
            log("too many closers", "result");
            step("impossible", `Even reading every star as an opener leaves ${hi} — there are more closers here than anything could match.`,
                [ array("string", cs).bad([i]),
                  text("possible open counts", [`from ${lo} to ${hi}`]) ]);
            step("no", "No assignment of stars can save it.", [ text("answer", ["false"]) ], "return");
            break;
        }

        if lo < 0 {
            step("clamp", `lo went to ${lo}, but a negative number of open brackets is not a real possibility — it just means the pessimistic reading over-closed. Clamp it to 0.`,
                [ text("possible open counts", [`from 0 to ${hi}`]) ]);
            lo = 0;
            set(#{ lo: N(lo) });
        } else {
            step("clamp", `lo is ${lo}, which is a genuine possibility.`,
                [ text("possible open counts", [`from ${lo} to ${hi}`]) ]);
        }
    }

    if ok {
        ok = lo == 0;
        log(`answer ${ok}`, "result");
        step("ret",
            if ok {
                "The range of possible open counts still includes 0, so some reading of the stars balances the string."
            } else {
                `Even the most closing-friendly reading leaves ${lo} bracket(s) open.`
            },
            [ array("string", cs).done(seq(0, n - 1)),
              text("possible open counts", [`from ${lo} to ${hi}`]) ], "return");
    }

    result(`${ok}`);
    leave(`${ok}`);
}
