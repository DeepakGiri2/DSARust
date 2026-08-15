// Insert Interval — three contiguous phases, one pass.
//
// The input being sorted and non-overlapping is what makes this work: the
// intervals entirely before the new one form a run, the overlapping ones form
// the next run, and the rest form the last. So each is handled by a simple
// while loop rather than a search.

fn parse(spec) {
    let out = [];
    for pair in words(spec) {
        let parts = pair.split(",");
        if parts.len() == 2 { out.push([num(parts[0]), num(parts[1])]); }
    }
    out
}

fn labels(list) {
    let out = [];
    for iv in list { out.push(`${iv[0]}-${iv[1]}`); }
    out
}

fn trace(input) {
    let intervals = parse(input.intervals);
    let fresh = parse(input.newInterval)[0];
    let n = intervals.len();
    let res = [];
    let i = 0;

    enter("insert", #{ newInterval: S(`${fresh[0]}-${fresh[1]}`) });
    step("init", `Insert [${fresh[0]}, ${fresh[1]}] into a sorted, non-overlapping list.`,
        [ array("intervals", labels(intervals)), array("result", labels(res)) ]);
    set(#{ i: N(i) });
    step("idx", "Walk the list once.",
        [ array("intervals", labels(intervals)).ptr("i", i) ]);

    while i < n && intervals[i][1] < fresh[0] {
        step("before", `[${intervals[i][0]}, ${intervals[i][1]}] ends before ${fresh[0]}, so it cannot touch the new interval.`,
            [ array("intervals", labels(intervals)).ptr("i", i).hl([i]),
              array("result", labels(res)) ]);
        res.push(intervals[i]);
        step("keep", "Copy it across unchanged.",
            [ array("intervals", labels(intervals)).ptr("i", i),
              array("result", labels(res)).hl([res.len() - 1]) ]);
        i += 1;
        set(#{ i: N(i) });
        step("advBefore", `i = ${i}.`,
            [ array("intervals", labels(intervals)).ptr("i", i).done(before(i)) ]);
    }

    while i < n && intervals[i][0] <= fresh[1] {
        step("overlap", `[${intervals[i][0]}, ${intervals[i][1]}] starts at or before ${fresh[1]}, so it overlaps and must be absorbed.`,
            [ array("intervals", labels(intervals)).ptr("i", i).hl([i]),
              text("growing interval", [`${fresh[0]}-${fresh[1]}`]) ]);

        if intervals[i][0] < fresh[0] {
            fresh[0] = intervals[i][0];
            step("widenLo", `It starts earlier, so the merged interval now begins at ${fresh[0]}.`,
                [ text("growing interval", [`${fresh[0]}-${fresh[1]}`]) ]);
        } else {
            step("widenLo", "It does not start earlier.",
                [ text("growing interval", [`${fresh[0]}-${fresh[1]}`]) ]);
        }
        if intervals[i][1] > fresh[1] {
            fresh[1] = intervals[i][1];
            step("widenHi", `It ends later, so the merged interval now runs to ${fresh[1]}.`,
                [ text("growing interval", [`${fresh[0]}-${fresh[1]}`]) ]);
        } else {
            step("widenHi", "It does not end later.",
                [ text("growing interval", [`${fresh[0]}-${fresh[1]}`]) ]);
        }

        i += 1;
        set(#{ i: N(i) });
        step("advOverlap", `i = ${i}.`,
            [ array("intervals", labels(intervals)).ptr("i", i).done(before(i)) ]);
    }

    res.push(fresh);
    log(`merged interval ${fresh[0]}-${fresh[1]}`, "result");
    step("place", `Place the merged interval [${fresh[0]}, ${fresh[1]}].`,
        [ array("result", labels(res)).hl([res.len() - 1]) ]);

    while i < n {
        step("after", `[${intervals[i][0]}, ${intervals[i][1]}] starts after the merged interval ends.`,
            [ array("intervals", labels(intervals)).ptr("i", i).hl([i]) ]);
        res.push(intervals[i]);
        step("tail", "Copy it across.",
            [ array("result", labels(res)).hl([res.len() - 1]) ]);
        i += 1;
    }

    let answer = join(labels(res), " ");
    result(answer);
    step("ret", `${answer} — still sorted and still non-overlapping.`,
        [ array("result", labels(res)).hl(seq(0, res.len() - 1)) ], "return");
    leave(answer);
}
