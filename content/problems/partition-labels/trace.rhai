// Partition Labels — a cut is safe once the scan reaches the furthest last
// occurrence.
//
// The first pass turns "does this letter appear again?" into a lookup. The
// second pass then only has to keep one number: the furthest index any letter
// in the current part reaches. Arriving at that number means nothing inside
// the part appears later, so the cut cannot split a letter.

fn trace(input) {
    let cs = chars(input.s);
    let n = cs.len();
    let last = map_new();

    enter("partitionLabels", #{ s: S(input.s) });
    step("last", "First, where does each letter appear for the last time?",
        [ array("s", cs), kv("last index", last) ]);

    for i in 0..n {
        step("scan", `Position ${i} holds '${cs[i]}'.`,
            [ array("s", cs).ptr("i", i).hl([i]).done(before(i)), kv("last index", last) ]);
        map_set(last, cs[i], i);
        step("record", `last['${cs[i]}'] = ${i} — later occurrences simply overwrite it.`,
            [ array("s", cs).ptr("i", i).done(before(i)), kv("last index", last).hl([cs[i]]) ]);
    }

    let res = [];
    set(#{ res: A(res) });
    step("init", "Now sweep once more, cutting where it is safe.",
        [ array("s", cs), kv("last index", last) ]);

    let start = 0;
    let end = 0;
    set(#{ start: N(start), end: N(end) });
    step("bounds", "start and end delimit the part being built.",
        [ array("s", cs).pointers(#{ start: start, end: end }) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `i = ${i}, letter '${cs[i]}'.`,
            [ array("s", cs).pointers(#{ i: i, end: end }).window(start, end).done(before(start)),
              kv("last index", last) ]);

        let l = map_get(last, cs[i]);
        if l > end {
            step("extend", `'${cs[i]}' appears again at ${l}, beyond the current boundary ${end} — the part has to stretch.`,
                [ array("s", cs).pointers(#{ i: i, end: end }).hl([i, l]),
                  kv("last index", last).hl([cs[i]]) ]);
            end = l;
            set(#{ end: N(end) });
            step("setEnd", `The part now runs to at least ${end}.`,
                [ array("s", cs).pointers(#{ i: i, end: end }).window(start, end) ]);
        } else {
            step("extend", `'${cs[i]}' last appears at ${l}, already inside the part.`,
                [ array("s", cs).pointers(#{ i: i, end: end }).window(start, end) ]);
        }

        if i == end {
            step("cut", `The scan has reached ${end}: no letter in [${start}..${end}] appears after it, so cutting here splits nothing.`,
                [ array("s", cs).hl(span(start, end)) ]);
            res.push(end - start + 1);
            set(#{ res: A(res) });
            log(`part [${start}..${end}] of length ${end - start + 1}`, "result");
            step("emit", `Emit a part of length ${end - start + 1}.`,
                [ array("s", cs).hl(span(start, end)), array("sizes", res).hl([res.len() - 1]) ]);
            start = i + 1;
            set(#{ start: N(start) });
            step("next", `The next part starts at ${start}.`,
                [ array("s", cs).ptr("start", start).done(before(start)) ]);
        }
    }

    let answer = join(res, " ");
    result(answer);
    step("ret", `${plural(res.len(), "part")}: ${answer}.`,
        [ array("s", cs), array("sizes", res).hl(seq(0, res.len() - 1)) ], "return");
    leave(answer);
}
