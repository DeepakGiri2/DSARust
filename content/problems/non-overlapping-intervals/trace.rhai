// Non-overlapping Intervals — sort by *end*, then keep greedily.
//
// Sorting by end time is the whole idea. When two intervals clash, the one
// that finishes later is never the better one to keep: it blocks at least as
// much of what follows. So the greedy choice is provably optimal and a single
// pass counts the removals.

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
    let n = intervals.len();

    for i in 1..n {
        let j = i;
        while j > 0 && intervals[j - 1][1] > intervals[j][1] {
            swap_at(intervals, j - 1, j);
            j -= 1;
        }
    }

    enter("eraseOverlapIntervals", #{ count: N(n) });
    step("sort",
        `Sort by end time: ${join(labels(intervals), " ")}. Finishing early leaves the most room for what comes next.`,
        [ array("intervals, by end time", labels(intervals)) ]);

    let removed = 0;
    set(#{ removed: N(removed) });
    step("init", "Nothing removed yet.", [ array("intervals, by end time", labels(intervals)) ]);

    let end = intervals[0][1];
    let kept = [0];
    set(#{ end: N(end) });
    step("first", `Keep the first one, [${intervals[0][0]}, ${intervals[0][1]}] — it ends soonest, so it is always safe.`,
        [ array("intervals, by end time", labels(intervals)).hl(kept) ]);

    for i in 1..n {
        set(#{ i: N(i) });
        step("loop", `Interval ${i} is [${intervals[i][0]}, ${intervals[i][1]}].`,
            [ array("intervals, by end time", labels(intervals)).ptr("i", i).hl(kept) ]);

        if intervals[i][0] < end {
            step("clash", `It starts at ${intervals[i][0]}, before the kept interval ends at ${end} — they overlap.`,
                [ array("intervals, by end time", labels(intervals)).ptr("i", i).bad([i]).hl(kept) ]);
            removed += 1;
            set(#{ removed: N(removed) });
            log(`remove ${intervals[i][0]}-${intervals[i][1]}`);
            step("drop",
                `Remove this one rather than the kept one: it ends later, so keeping it could only block more of what follows. Removed: ${removed}.`,
                [ array("intervals, by end time", labels(intervals)).bad([i]).hl(kept) ]);
        } else {
            step("clash", `It starts at ${intervals[i][0]}, at or after ${end} — no overlap.`,
                [ array("intervals, by end time", labels(intervals)).ptr("i", i).hl(kept) ]);
            end = intervals[i][1];
            kept.push(i);
            set(#{ end: N(end) });
            step("keep", `Keep it; the frontier moves to ${end}.`,
                [ array("intervals, by end time", labels(intervals)).hl(kept) ]);
        }
    }

    log(`answer ${removed}`, "result");
    result(`${removed}`);
    step("ret", `Removing ${plural(removed, "interval")} leaves ${kept.len()} that do not overlap.`,
        [ array("intervals, by end time", labels(intervals)).hl(kept) ], "return");
    leave(`${removed}`);
}
