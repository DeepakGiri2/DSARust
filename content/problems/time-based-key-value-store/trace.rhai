// Time Based Key-Value Store — binary search for the rightmost entry at or
// before a time.
//
// Because timestamps only ever increase, each key's history is sorted without
// any sorting. The get is the "largest value <= target" variant: on a match,
// remember it and keep searching *right*, because a later entry that still
// qualifies would be a better answer.

fn history_labels(hist) {
    let out = [];
    for e in hist { out.push(`${e[0]}:${e[1]}`); }
    out
}

fn trace(input) {
    let store = map_new();
    let out = [];

    enter("TimeMap", #{});
    step("init", "One history per key, kept in timestamp order.", [ kv("store", store) ]);

    for op in input.ops.split(",") {
        let parts = words(op);
        if parts.len() == 0 { continue; }

        if parts[0] == "set" {
            let key = parts[1];
            let value = parts[2];
            let stamp = num(parts[3]);
            let hist = if map_has(store, key) { map_get(store, key) } else { [] };
            hist.push([stamp, value]);
            map_set(store, key, hist);
            log(`set ${key} = ${value} at ${stamp}`);
            step("set",
                `set("${key}", "${value}", ${stamp}). Timestamps only increase, so appending keeps the history sorted for free.`,
                [ array(`history of "${key}"`, history_labels(hist)).hl([hist.len() - 1]) ]);
        } else {
            let key = parts[1];
            let stamp = num(parts[2]);
            enter(`get("${key}", ${stamp})`, #{ stamp: N(stamp) });

            let hist = if map_has(store, key) { map_get(store, key) } else { [] };
            step("hist", `The history of "${key}" holds ${hist.len()} entr(y/ies).`,
                [ array(`history of "${key}"`, history_labels(hist)) ]);

            let lo = 0;
            let hi = hist.len() - 1;
            let best = "";
            set(#{ lo: N(lo), hi: N(hi) });
            step("bounds", "Search the whole history.",
                [ array(`history of "${key}"`, history_labels(hist)).window(lo, hi) ]);
            step("best", "No answer yet — an empty string means nothing was set in time.",
                [ array(`history of "${key}"`, history_labels(hist)) ]);

            while lo <= hi {
                step("loop", `Live range [${lo}..${hi}].`,
                    [ array(`history of "${key}"`, history_labels(hist))
                        .pointers(#{ lo: lo, hi: hi }).window(lo, hi) ]);

                let mid = lo + (hi - lo) / 2;
                step("mid", `mid = ${mid}, stamped ${hist[mid][0]}.`,
                    [ array(`history of "${key}"`, history_labels(hist))
                        .pointers(#{ lo: lo, mid: mid, hi: hi }).hl([mid]).window(lo, hi) ]);

                if hist[mid][0] <= stamp {
                    best = hist[mid][1];
                    step("cmp", `${hist[mid][0]} <= ${stamp}, so "${best}" was current at that time — a candidate.`,
                        [ array(`history of "${key}"`, history_labels(hist)).hl([mid]) ]);
                    step("keep", `Remember "${best}".`,
                        [ array(`history of "${key}"`, history_labels(hist)).hl([mid]) ]);
                    lo = mid + 1;
                    step("later",
                        `Keep searching to the right: a later entry that still qualifies would be more recent, and therefore better.`,
                        [ array(`history of "${key}"`, history_labels(hist))
                            .pointers(#{ lo: lo, hi: hi }).window(lo, hi) ]);
                } else {
                    step("cmp", `${hist[mid][0]} > ${stamp} — that value had not been set yet.`,
                        [ array(`history of "${key}"`, history_labels(hist)).bad([mid]) ]);
                    hi = mid - 1;
                    step("earlier", `Look earlier: hi = ${hi}.`,
                        [ array(`history of "${key}"`, history_labels(hist))
                            .pointers(#{ lo: lo, hi: hi }).window(lo, hi) ]);
                }
            }

            out.push(best);
            log(`get("${key}", ${stamp}) -> "${best}"`, "result");
            step("ret", `Return "${best}".`,
                [ array(`history of "${key}"`, history_labels(hist)), text("output", out) ],
                "return");
            leave(best);
        }
    }

    let answer = join(out, " ");
    result(answer);
}
