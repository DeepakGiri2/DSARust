// Restore IP Addresses — both rules are breaks, not skips.
//
// A leading zero rules out every *longer* segment starting here, not just this
// one, because they all begin with that same zero. And segments only grow as
// the length increases, so once one passes 255 the rest do too. Both
// conditions therefore end the loop rather than skipping an iteration — a
// small distinction that removes most of the search space.
//
// The final "did we consume everything?" test matters as much: four perfectly
// valid segments that leave digits behind are not an address.

fn seg_of(cs, start, length) {
    let out = "";
    let k = start;
    while k < start + length { out += cs[k]; k += 1; }
    out
}

fn back(cs, start, parts, res) {
    enter(`back(start = ${start})`, #{ start: N(start), parts: N(parts.len()) });

    if parts.len() == 4 {
        step("four", "Four segments have been chosen.",
            [ array("digits", cs).ptr("start", start).done(before(start)),
              text("segments", [join(parts, " . ")]) ]);

        if start == cs.len() {
            step("consumed", "And the string is fully consumed, so this really is an address.",
                [ array("digits", cs).done(seq(0, cs.len() - 1)) ]);
            res.push(join(parts, "."));
            log(`found ${join(parts, ".")}`, "result");
            step("record", `Record ${join(parts, ".")}.`, [ text("addresses", res) ]);
        } else {
            step("consumed", `${cs.len() - start} digit(s) are left over, so these four segments do not use the whole string.`,
                [ array("digits", cs).ptr("start", start).bad(seq(start, cs.len() - 1)) ]);
            step("record", "Nothing to record.", [ text("addresses", res) ]);
        }

        step("retDone", "Unwind.", [ text("addresses", res) ], "return");
        leave("");
        return res;
    }

    step("four", `${parts.len()} of 4 segments chosen.`,
        [ array("digits", cs).ptr("start", start).done(before(start)),
          text("segments", [join(parts, " . ")]) ]);

    let length = 1;
    while length <= 3 && start + length <= cs.len() {
        step("loop", `Try a ${length}-digit segment.`,
            [ array("digits", cs).pointers(#{ start: start })
                .hl(span(start, start + length - 1)).done(before(start)) ]);

        let seg = seg_of(cs, start, length);
        step("slice", `That gives "${seg}".`,
            [ array("digits", cs).hl(span(start, start + length - 1)) ]);

        if seg.len() > 1 && cs[start] == "0" {
            step("leadingZero", `"${seg}" has a leading zero. So would every longer segment starting here, since they all begin with the same 0 — stop rather than skip.`,
                [ array("digits", cs).bad(span(start, cs.len() - 1)) ]);
            break;
        }

        if num(seg) > 255 {
            step("tooBig", `${num(seg)} is above 255, and a longer segment is only larger — stop.`,
                [ array("digits", cs).bad(span(start, start + length - 1)) ]);
            break;
        }

        parts.push(seg);
        step("choose", `Take "${seg}": ${join(parts, " . ")}.`,
            [ array("digits", cs).hl(span(start, start + length - 1)).done(before(start)),
              text("segments", [join(parts, " . ")]) ]);

        step("recurse", `Continue from digit ${start + length}.`,
            [ array("digits", cs).ptr("start", start + length) ], "call");
        res = back(cs, start + length, parts, res);

        parts.truncate(parts.len() - 1);
        step("undo", `Undo, and try a longer segment.`,
            [ text("segments", [join(parts, " . ")]) ]);

        length += 1;
    }

    leave("");
    res
}

fn trace(input) {
    let cs = chars(input.s);

    enter("restoreIpAddresses", #{ n: N(cs.len()) });
    step("init", "Valid addresses found.", [ array("digits", cs), text("addresses", []) ]);
    step("parts", "The segments chosen so far.", [ text("segments", [""]) ]);

    step("call", "Start at digit 0 with no segments chosen.", [ array("digits", cs) ], "call");
    let res = back(cs, 0, [], []);

    let answer = join(res, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(res.len(), "valid address")}.`, [ text("addresses", res) ], "return");
    leave(answer);
}
