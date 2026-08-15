fn parse_pairs(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

fn labels(ivs) {
    let out = [];
    for iv in ivs { out.push(`[${iv[0]},${iv[1]}]`); }
    out
}

fn sort_by_start(ivs) {
    for i in 1..ivs.len() {
        let j = i;
        while j > 0 && ivs[j - 1][0] > ivs[j][0] {
            swap_at(ivs, j - 1, j);
            j -= 1;
        }
    }
    ivs
}

// Minimum Interval per Query — reorder the queries, then sweep once.
//
// Taken in the given order, each query would need its own search. Taken in
// increasing order, the whole thing becomes a sweep: a cursor admits every
// interval that has started, and a heap ordered by size offers the smallest of
// them.
//
// The expiry loop only ever inspects the heap's *top*, which is not obviously
// enough — an expired interval could be buried deeper. It does not matter: a
// buried interval is larger than the top, so it can only become the answer
// after everything above it has been popped, and it will be checked then. And
// since the queries only move forwards, an expired interval is expired for
// good.

fn hpush(h, key, end) {
    h.push([key, end]);
    let i = h.len() - 1;
    while i > 0 {
        let p = (i - 1) / 2;
        if h[p][0] <= h[i][0] { break; }
        swap_at(h, p, i);
        i = p;
    }
    h
}

fn hpop(h) {
    let n = h.len();
    h[0] = h[n - 1];
    h.truncate(n - 1);
    let i = 0;
    loop {
        let l = 2 * i + 1;
        let r = 2 * i + 2;
        let small = i;
        if l < h.len() && h[l][0] < h[small][0] { small = l; }
        if r < h.len() && h[r][0] < h[small][0] { small = r; }
        if small == i { break; }
        swap_at(h, i, small);
        i = small;
    }
    h
}

fn heap_view(h) {
    let out = [];
    for e in h { out.push(`size ${e[0]}, ends ${e[1]}`); }
    out
}

fn trace(input) {
    let ivs = parse_pairs(input.intervals);
    let queries = input.queries;
    let q = queries.len();

    enter("minInterval", #{ intervals: N(ivs.len()), queries: N(q) });

    ivs = sort_by_start(ivs);
    step("sortIntervals", "Intervals sorted by start, so one forward cursor can release them.",
        [ array("intervals (sorted by start)", labels(ivs)) ]);

    let order = [];
    for i in 0..q { order.push(i); }
    for i in 1..q {
        let j = i;
        while j > 0 && queries[order[j - 1]] > queries[order[j]] {
            swap_at(order, j - 1, j);
            j -= 1;
        }
    }
    step("sortQueries", "Queries answered in increasing order, but their answers are written back to the original positions.",
        [ array("queries", queries), array("answer order (indices)", order) ]);

    let h = [];
    step("init", "A min-heap of live intervals, keyed by size.",
        [ heap("live intervals (smallest first)", heap_view(h)) ]);

    let res = [];
    for i in 0..q { res.push(0); }
    step("out", "One answer per query.", [ array("answers", res) ]);

    let i = 0;
    set(#{ i: N(i) });
    step("cursor", "The cursor into the sorted intervals.",
        [ array("intervals (sorted by start)", labels(ivs)).ptr("i", i) ]);

    for k in 0..q {
        let qi = order[k];
        step("loop", `Query ${k + 1} of ${q} in sorted order.`,
            [ array("queries", queries).ptr("qi", qi),
              heap("live intervals (smallest first)", heap_view(h)) ]);

        let x = queries[qi];
        step("query", `The point is ${x} (originally query ${qi}).`,
            [ array("queries", queries).ptr("qi", qi).hl([qi]) ]);

        while i < ivs.len() && ivs[i][0] <= x {
            step("admit", `${labels(ivs)[i]} starts at ${ivs[i][0]}, which is at or before ${x} — so it is now in play, and stays in play for every later query.`,
                [ array("intervals (sorted by start)", labels(ivs)).ptr("i", i).hl([i]) ]);
            h = hpush(h, ivs[i][1] - ivs[i][0] + 1, ivs[i][1]);
            step("push", `Its size is ${ivs[i][1] - ivs[i][0] + 1}.`,
                [ heap("live intervals (smallest first)", heap_view(h)) ]);
            i += 1;
            set(#{ i: N(i) });
        }

        while h.len() > 0 && h[0][1] < x {
            step("expire", `The smallest live interval ends at ${h[0][1]}, before ${x}. Since the queries only move forwards, it can never be useful again.`,
                [ heap("live intervals (smallest first)", heap_view(h)) ]);
            h = hpop(h);
            step("drop", "Discard it.",
                [ heap("live intervals (smallest first)", heap_view(h)) ]);
        }

        if h.len() == 0 {
            res[qi] = -1;
            log(`query ${x} -> -1`, "result");
            step("none", `Nothing live contains ${x}.`,
                [ heap("live intervals (smallest first)", heap_view(h)) ]);
            step("miss", "So the answer is -1.", [ array("answers", res).hl([qi]) ]);
        } else {
            res[qi] = h[0][0];
            log(`query ${x} -> ${h[0][0]}`, "result");
            step("none", `The heap's top is size ${h[0][0]}, ending at ${h[0][1]} — it started before ${x} and has not ended, so it contains ${x}.`,
                [ heap("live intervals (smallest first)", heap_view(h)) ]);
            step("hit", `Smallest containing interval has size ${res[qi]}.`,
                [ array("answers", res).hl([qi]) ]);
        }
    }

    let answer = join(res, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Answers in the original query order: ${answer}.`,
        [ array("queries", queries), array("answers", res) ], "return");
    leave(answer);
}
