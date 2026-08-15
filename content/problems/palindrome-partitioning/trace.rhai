// Palindrome Partitioning — filter as you choose, not afterwards.
//
// The recursion is over cut positions rather than characters: from `start`,
// every possible first piece is tried, and only the palindromic ones are
// entered. Testing at the moment of choosing is what makes this a *prune* — a
// non-palindromic first piece kills every partition that would have contained
// it, without any of them being built. Reaching the end of the string is then
// automatically a complete, valid answer.

fn is_pal(cs, lo, hi) {
    while lo < hi {
        if cs[lo] != cs[hi] { return false; }
        lo += 1;
        hi -= 1;
    }
    true
}

fn piece(cs, lo, hi) {
    let out = "";
    let k = lo;
    while k <= hi { out += cs[k]; k += 1; }
    out
}

fn back(cs, start, cur, res) {
    enter(`back(start = ${start})`, #{ start: N(start) });

    if start == cs.len() {
        step("complete", "The whole string has been consumed, and every piece taken was a palindrome.",
            [ array("string", cs).done(seq(0, cs.len() - 1)),
              text("current partition", [join(cur, " | ")]) ]);
        res.push(join(cur, "|"));
        log(`found ${join(cur, "|")}`, "result");
        step("record", `Record ${join(cur, " | ")}.`, [ text("partitions", res) ]);
        step("retDone", "Unwind.", [ text("partitions", res) ], "return");
        leave("");
        return res;
    }

    step("complete", `${start} of ${cs.len()} characters consumed; the next piece starts here.`,
        [ array("string", cs).ptr("start", start).done(before(start)),
          text("current partition", [join(cur, " | ")]) ]);

    for end in start..cs.len() {
        step("loop", `Try "${piece(cs, start, end)}" as the next piece.`,
            [ array("string", cs).pointers(#{ start: start, end: end })
                .hl(span(start, end)).done(before(start)) ]);

        if !is_pal(cs, start, end) {
            step("check", `"${piece(cs, start, end)}" is not a palindrome, so every partition beginning with it is dead — and none of them gets built.`,
                [ array("string", cs).bad(span(start, end)) ]);
            continue;
        }
        step("check", `"${piece(cs, start, end)}" reads the same both ways.`,
            [ array("string", cs).hl(span(start, end)) ]);

        cur.push(piece(cs, start, end));
        step("choose", `Cut it off: ${join(cur, " | ")}.`,
            [ array("string", cs).hl(span(start, end)).done(before(start)),
              text("current partition", [join(cur, " | ")]) ]);

        step("recurse", `Partition the rest, starting at ${end + 1}.`,
            [ array("string", cs).ptr("start", end + 1).done(before(end + 1)) ], "call");
        res = back(cs, end + 1, cur, res);

        cur.truncate(cur.len() - 1);
        step("undo", `Undo that cut and try a longer first piece.`,
            [ text("current partition", [join(cur, " | ")]) ]);
    }

    leave("");
    res
}

fn trace(input) {
    let cs = chars(input.s);

    enter("partition", #{ n: N(cs.len()) });
    step("init", "Every valid partition found.", [ array("string", cs), text("partitions", []) ]);
    step("cur", "The pieces cut off so far.", [ text("current partition", [""]) ]);

    step("call", "Start at position 0.", [ array("string", cs) ], "call");
    let res = back(cs, 0, [], []);

    let answer = join(res, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(res.len(), "partition")}.`, [ text("partitions", res) ], "return");
    leave(answer);
}
