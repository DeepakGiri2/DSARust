// Combinations — increasing order removes the duplicates for free.
//
// A combination has no order, so generating {1,3} and {3,1} separately would
// be wrong. Insisting that each choice is larger than the last means only one
// of the two orderings is ever built — no set of already-seen results is
// needed. The loop bound is the second idea: if there are not enough numbers
// left above v to reach k in total, the whole branch is hopeless and is never
// started.

fn back(n, k, start, cur, res) {
    enter(`back(start = ${start})`, #{ start: N(start) });

    if cur.len() == k {
        step("complete", `The combination has ${k} number(s), so it is finished.`,
            [ array("current combination", cur), text("results", res) ]);
        res.push(join(cur, ","));
        log(`found ${join(cur, ",")}`, "result");
        step("record", `Record {${join(cur, ", ")}} — result ${res.len()}.`,
            [ text("results", res) ]);
        step("retDone", "Unwind and try a different choice.", [ text("results", res) ], "return");
        leave("");
        return res;
    }

    let limit = n - (k - cur.len()) + 1;
    step("complete", `${cur.len()} of ${k} chosen so far; ${k - cur.len()} still needed.`,
        [ array("current combination", cur), array("available values", seq(start, n)),
          text("results", res) ]);

    let v = start;
    while v <= limit {
        step("loop", `Values ${start}..${limit} are worth trying. Anything above ${limit} leaves fewer than ${k - cur.len()} numbers above it, so it could never be completed.`,
            [ array("values 1..n", seq(1, n)).hl(seq(v - 1, limit - 1)).bad(seq(limit, n - 1)),
              array("current combination", cur) ]);

        cur.push(v);
        step("choose", `Take ${v}.`,
            [ array("values 1..n", seq(1, n)).hl([v - 1]),
              array("current combination", cur) ]);

        step("recurse", `Recurse — and only values above ${v} are allowed from here, which is what stops {${join(cur, ", ")}} being generated in another order.`,
            [ array("current combination", cur) ], "call");
        res = back(n, k, v + 1, cur, res);

        cur.truncate(cur.len() - 1);
        step("undo", `Put ${v} back and try the next value.`,
            [ array("current combination", cur) ]);

        v += 1;
    }

    leave("");
    res
}

fn trace(input) {
    let n = input.n;
    let k = input.k;

    enter("combine", #{ n: N(n), k: N(k) });
    step("init", `Every way of choosing ${k} number(s) from 1..${n}.`,
        [ array("values 1..n", seq(1, n)) ]);
    step("cur", "The combination being built.", [ array("current combination", []) ]);

    step("call", "Start with nothing chosen and 1 as the smallest allowed value.",
        [ array("values 1..n", seq(1, n)) ], "call");
    let res = back(n, k, 1, [], []);

    let answer = join(res, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(res.len(), "combination")}.`, [ text("results", res) ], "return");
    leave(answer);
}
