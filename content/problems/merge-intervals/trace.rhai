// Merge Intervals — sorting first turns a pairwise problem into one pass.
//
// Once the intervals are ordered by start, only the *last* merged interval can
// possibly overlap the next one. That is why a single scan suffices, and it is
// what the two-row picture is meant to show.

fn parse_intervals(spec) {
    let out = [];
    for pair in words(spec) {
        let parts = pair.split(",");
        if parts.len() == 2 { out.push([num(parts[0]), num(parts[1])]); }
    }
    out
}

fn as_labels(list) {
    let out = [];
    for iv in list { out.push(`${iv[0]}-${iv[1]}`); }
    out
}

fn trace(input) {
    let intervals = parse_intervals(input.intervals);
    let n = intervals.len();

    enter("merge", #{ count: N(n) });
    step("sort", "First sort by start time. Without that, an interval could overlap something far behind it and a single pass would miss it.",
        [ array("input order", as_labels(intervals)) ]);

    // Insertion sort, so the reordering is something the viewer can follow.
    for i in 1..n {
        let j = i;
        while j > 0 && intervals[j - 1][0] > intervals[j][0] {
            swap_at(intervals, j - 1, j);
            j -= 1;
        }
    }
    step("sort", `Sorted by start: ${join(as_labels(intervals), ", ")}.`,
        [ array("sorted by start", as_labels(intervals)) ]);

    let merged = [];
    set(#{ merged: A(as_labels(merged)) });
    step("init", "Nothing merged yet.",
        [ array("sorted by start", as_labels(intervals)),
          array("merged", as_labels(merged)) ]);

    for i in 0..n {
        let cur = intervals[i];
        set(#{ i: N(i), cur: S(`${cur[0]}-${cur[1]}`) });
        step("loop", `Interval ${i}: [${cur[0]}, ${cur[1]}].`,
            [ array("sorted by start", as_labels(intervals)).ptr("i", i).done(before(i)),
              array("merged", as_labels(merged)) ]);

        if merged.len() == 0 || merged[merged.len() - 1][1] < cur[0] {
            step("disjoint",
                if merged.len() == 0 {
                    "Nothing to merge with yet.".to_string()
                } else {
                    `The last merged interval ends at ${merged[merged.len() - 1][1]}, before this one starts at ${cur[0]} — they do not touch.`
                },
                [ array("sorted by start", as_labels(intervals)).ptr("i", i).hl([i]).done(before(i)),
                  array("merged", as_labels(merged)) ]);
            merged.push([cur[0], cur[1]]);
            set(#{ merged: A(as_labels(merged)) });
            log(`start a new interval [${cur[0]}, ${cur[1]}]`);
            step("append", `Start a new merged interval [${cur[0]}, ${cur[1]}].`,
                [ array("sorted by start", as_labels(intervals)).ptr("i", i).done(before(i)),
                  array("merged", as_labels(merged)).hl([merged.len() - 1]) ]);
        } else if cur[1] > merged[merged.len() - 1][1] {
            let last = merged[merged.len() - 1];
            step("overlap",
                `[${cur[0]}, ${cur[1]}] starts at or before ${last[1]}, so it overlaps the last merged interval [${last[0]}, ${last[1]}] — and it reaches further.`,
                [ array("sorted by start", as_labels(intervals)).ptr("i", i).hl([i]).done(before(i)),
                  array("merged", as_labels(merged)).hl([merged.len() - 1]) ]);
            merged[merged.len() - 1][1] = cur[1];
            set(#{ merged: A(as_labels(merged)) });
            log(`extend to ${cur[1]}`);
            step("extend", `Stretch it to [${last[0]}, ${cur[1]}].`,
                [ array("sorted by start", as_labels(intervals)).ptr("i", i).done(before(i)),
                  array("merged", as_labels(merged)).hl([merged.len() - 1]) ]);
        } else {
            step("overlap",
                `[${cur[0]}, ${cur[1]}] is completely swallowed by the last merged interval — nothing changes.`,
                [ array("sorted by start", as_labels(intervals)).ptr("i", i).done(before(i)),
                  array("merged", as_labels(merged)).hl([merged.len() - 1]) ]);
        }
    }

    let answer = `[${join(as_labels(merged), ", ")}]`;
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${n} interval(s) collapsed into ${merged.len()}: ${answer}.`,
        [ array("merged", as_labels(merged)).hl(seq(0, merged.len() - 1)) ], "return");
    leave(answer);
}
