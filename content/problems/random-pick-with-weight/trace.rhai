// Random Pick with Weight — weights as segments of a number line.
//
// Weight 3 means "own three of the numbers", so the prefix sums are just the
// right-hand endpoints of each index's segment. A uniform draw lands on one
// number, and finding its owner is a lower-bound binary search: the first
// prefix value that has not yet been passed.
//
// The animation cannot roll dice — a trace has to be reproducible — so the
// draws are supplied as input. Each one is a value the real rand() could have
// returned, and the search that follows is exactly the code's.

const W = "w — weights";
const PREFIX = "prefix — the last number each index owns";

fn parse_draws(spec) {
    let out = [];
    for part in spec.split(",") {
        let t = part;
        t.trim();
        if t != "" { out.push(num(t)); }
    }
    out
}

fn segments(w, prefix) {
    let out = [];
    let lo = 1;
    for i in 0..w.len() {
        out.push(`index ${i}: ${lo} … ${prefix[i]}   (weight ${w[i]})`);
        lo = prefix[i] + 1;
    }
    out
}

fn trace(input) {
    let w = input.w;
    let n = w.len();
    let draws = parse_draws(input.draws);

    enter("Constructor", #{ w: A(w) });

    let prefix = [];
    for i in 0..n { prefix.push(0); }
    step("init", "One prefix array. That is the entire structure — the randomness lives in the query, not the data.",
        [ array(global::W, w), array(global::PREFIX, prefix) ]);

    let sum = 0;
    set(#{ sum: N(0) });
    step("zero", "Running total starts at 0.", [ array(global::W, w) ]);

    for i in 0..n {
        step("build", `Weight ${w[i]} at index ${i}.`,
            [ array(global::W, w).ptr("i", i).done(before(i)).hl([i]),
              array(global::PREFIX, prefix).done(before(i)) ]);
        sum += w[i];
        prefix[i] = sum;
        set(#{ sum: N(sum), prefix: A(prefix) });
        step("add", `prefix[${i}] = ${sum} — index ${i} owns every draw up to and including ${sum}.`,
            [ array(global::W, w).ptr("i", i).done(before(i)),
              array(global::PREFIX, prefix).hl([i]).done(before(i)) ]);
    }

    let total = prefix[n - 1];
    step("ready", `Total weight ${total}. The number line 1 … ${total} is now carved into ${plural(n, "segment")}.`,
        [ array(global::PREFIX, prefix), text("who owns what", segments(w, prefix)) ],
        "return");
    leave("built");

    let out = [];
    for d in draws {
        enter("pickIndex");
        step("pick", "pickIndex().", [ array(global::PREFIX, prefix) ]);

        set(#{ target: N(d) });
        step("draw", `The draw is ${d} — a uniform pick from 1 … ${total}. Every number is equally likely, so a segment ${plural(w[0], "number")} long is picked in proportion to its length.`,
            [ array(global::PREFIX, prefix), text("who owns what", segments(w, prefix)) ]);

        let lo = 0;
        let hi = n - 1;
        set(#{ lo: N(lo), hi: N(hi) });
        step("bounds", `Search the prefix array for the first value ≥ ${d}.`,
            [ array(global::PREFIX, prefix).ptr("lo", lo).ptr("hi", hi) ]);

        while lo < hi {
            step("search", `lo = ${lo}, hi = ${hi} — still more than one candidate.`,
                [ array(global::PREFIX, prefix).ptr("lo", lo).ptr("hi", hi).done(dead(n, lo, hi)) ]);

            let mid = (lo + hi) / 2;
            set(#{ mid: N(mid) });
            step("mid", `mid = ${mid}, prefix[${mid}] = ${prefix[mid]}.`,
                [ array(global::PREFIX, prefix).ptr("lo", lo).ptr("hi", hi).ptr("mid", mid)
                      .hl([mid]).done(dead(n, lo, hi)) ]);

            if prefix[mid] < d {
                step("cmp", `${prefix[mid]} < ${d}: index ${mid}'s segment ends before the draw, so the owner is further right.`,
                    [ array(global::PREFIX, prefix).ptr("mid", mid).bad([mid]).done(dead(n, lo, hi)) ]);
                lo = mid + 1;
                set(#{ lo: N(lo) });
                step("right", `lo = ${lo}.`,
                    [ array(global::PREFIX, prefix).ptr("lo", lo).ptr("hi", hi).done(dead(n, lo, hi)) ]);
            } else {
                step("cmp", `${prefix[mid]} ≥ ${d}: index ${mid} could be the owner, so it stays in the range.`,
                    [ array(global::PREFIX, prefix).ptr("mid", mid).hl([mid]).done(dead(n, lo, hi)) ]);
                hi = mid;
                set(#{ hi: N(hi) });
                step("left", `hi = ${hi} — note hi = mid, not mid − 1: discarding mid could discard the answer.`,
                    [ array(global::PREFIX, prefix).ptr("lo", lo).ptr("hi", hi).done(dead(n, lo, hi)) ]);
            }
        }

        out.push(`${lo}`);
        log(`draw ${d} → index ${lo}`, "result");
        step("ret", `lo = hi = ${lo}: the draw ${d} lands in index ${lo}'s segment.`,
            [ array(global::W, w).hl([lo]), array(global::PREFIX, prefix).hl([lo]),
              text("who owns what", segments(w, prefix)) ], "return");
        leave(`${lo}`);
    }

    let answer = join(out, " ");
    result(answer);
}
