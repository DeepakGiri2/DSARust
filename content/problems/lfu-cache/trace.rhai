// LFU Cache — two structures that must never disagree.
//
// LRU needs one ordering; LFU needs a count *and* an ordering within each
// count, and the whole difficulty is keeping them consistent in O(1). The
// bookkeeping to watch:
//
//   • every touch moves a key from bucket f to bucket f+1 — never a rescan;
//   • minFreq rises by exactly one when a touch empties the bucket it points
//     at (nothing can be left below it), and drops to 1 on any insert;
//   • eviction reads the tail of bucket minFreq, which is the least recently
//     used key among the least frequently used ones — both tie-breaks at once.
//
// If minFreq is ever wrong, eviction silently throws away the wrong key, and
// nothing else in the structure notices. That is the bug to look for.

const VALUES = "cache — key → value";
const FREQ = "use count of each key";

fn parse_ops(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if !w.is_empty() { out.push(w); }
    }
    out
}

fn bucket_lines(buckets, min_freq, size, cap) {
    let out = [];
    for f in map_keys(buckets) {
        let keys = map_get(buckets, f);
        if keys != "" {
            out.push(`freq ${f}: ${keys}${if f == min_freq { "   ← minFreq" } else { "" }}`);
        }
    }
    if out.is_empty() { out.push("(no buckets yet)"); }
    out.push(`size ${size} / ${cap}`);
    out
}

// Buckets are stored as comma-joined key lists, newest first.
fn bucket_keys(buckets, f) {
    if !map_has(buckets, f) { return []; }
    let s = map_get(buckets, f);
    s.replace(",", " ");
    words(s)
}

fn bucket_put(buckets, f, keys) {
    map_set(buckets, f, join(keys, ","));
    buckets
}

fn trace(input) {
    let cap = input.capacity;
    let ops = parse_ops(input.ops);

    enter("LFUCache", #{ capacity: N(cap) });

    let vals = map_new();       // key -> value
    let freq = map_new();       // key -> use count
    let buckets = map_new();    // use count -> "k1,k2" newest first
    let min_freq = 0;
    set(#{ minFreq: N(0), size: N(0) });
    step("init", `Capacity ${cap}. Two maps — key → value and key → use count — plus one bucket list per count, ordered by recency.`,
        [ kv(global::VALUES, vals), kv(global::FREQ, freq),
          text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);
    leave("ready");

    let out = [];
    for o in ops {
        let key = num(o[1]);

        if o[0] == "get" {
            enter(`get(${key})`, #{ key: N(key) });
            step("get", `get(${key}).`,
                [ kv(global::VALUES, vals).hl([key]), kv(global::FREQ, freq),
                  text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);

            if !map_has(vals, key) {
                out.push("-1");
                step("miss", `${key} is not in the cache.`,
                    [ kv(global::VALUES, vals), text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);
                step("nokey", "Return -1. A miss must not change any counts — it was never a use.",
                    [ kv(global::FREQ, freq) ], "return");
                leave("-1");
                continue;
            }

            step("hit", `Hit: ${key} → ${map_get(vals, key)}. A read counts as a use, so the frequency has to move.`,
                [ kv(global::VALUES, vals).hl([key]), kv(global::FREQ, freq).hl([key]) ]);

            // ── touch ───────────────────────────────────────────────────────
            let f = map_get(freq, key);
            enter(`touch(${key})`, #{ key: N(key), freq: N(f) });
            step("touch", `${key} currently sits in bucket ${f}.`,
                [ text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);

            let ks = bucket_keys(buckets, f);
            let keep = [];
            for k in ks { if num(k) != key { keep.push(k); } }
            buckets = bucket_put(buckets, f, keep);
            step("unlink", `Remove it from bucket ${f}.`,
                [ text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);

            if keep.is_empty() && min_freq == f {
                min_freq = f + 1;
                set(#{ minFreq: N(min_freq) });
                step("bumpmin", `Bucket ${f} is now empty and it was the minimum, so minFreq becomes ${min_freq}. Nothing can be below it: every key has been used at least ${min_freq} times.`,
                    [ text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);
            } else {
                step("bumpmin", `Bucket ${f} still has keys (or was not the minimum), so minFreq stays ${min_freq}.`,
                    [ text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);
            }

            f += 1;
            map_set(freq, key, f);
            step("bump", `${key}'s use count is now ${f}.`,
                [ kv(global::FREQ, freq).hl([key]) ]);

            let up = bucket_keys(buckets, f);
            up.insert(0, `${key}`);
            buckets = bucket_put(buckets, f, up);
            step("relink", `Put ${key} at the front of bucket ${f} — front means most recently used, which is how ties are broken later.`,
                [ text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ], "return");
            leave("");

            out.push(`${map_get(vals, key)}`);
            log(`get(${key}) = ${map_get(vals, key)}`, "result");
            step("val", `Return ${map_get(vals, key)}.`,
                [ kv(global::VALUES, vals).hl([key]), kv(global::FREQ, freq).hl([key]),
                  text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ], "return");
            leave(`${map_get(vals, key)}`);
            continue;
        }

        // ── put ─────────────────────────────────────────────────────────────
        let value = num(o[2]);
        enter(`put(${key}, ${value})`, #{ key: N(key), value: N(value) });
        step("put", `put(${key}, ${value}).`,
            [ kv(global::VALUES, vals), text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);

        if map_has(vals, key) {
            step("exists", `${key} is already cached — this is an overwrite, not an insert.`,
                [ kv(global::VALUES, vals).hl([key]) ]);
            map_set(vals, key, value);
            step("update", `Value updated to ${value}. The key does not move buckets yet…`,
                [ kv(global::VALUES, vals).hl([key]) ]);

            let f = map_get(freq, key);
            enter(`touch(${key})`, #{ key: N(key), freq: N(f) });
            step("touch", `…but a write is a use too, so ${key} moves out of bucket ${f}.`,
                [ text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);

            let ks = bucket_keys(buckets, f);
            let keep = [];
            for k in ks { if num(k) != key { keep.push(k); } }
            buckets = bucket_put(buckets, f, keep);
            step("unlink", `Removed from bucket ${f}.`,
                [ text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);

            if keep.is_empty() && min_freq == f {
                min_freq = f + 1;
                set(#{ minFreq: N(min_freq) });
                step("bumpmin", `Bucket ${f} emptied, so minFreq = ${min_freq}.`,
                    [ text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);
            } else {
                step("bumpmin", `minFreq stays ${min_freq}.`,
                    [ text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);
            }

            f += 1;
            map_set(freq, key, f);
            step("bump", `Use count ${f}.`, [ kv(global::FREQ, freq).hl([key]) ]);

            let up = bucket_keys(buckets, f);
            up.insert(0, `${key}`);
            buckets = bucket_put(buckets, f, up);
            step("relink", `Front of bucket ${f}.`,
                [ text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ], "return");
            leave("");
            leave("");
            continue;
        }

        if map_len(vals) == cap {
            step("full", `The cache is full (${map_len(vals)} of ${cap}), so something has to go.`,
                [ kv(global::VALUES, vals), text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);

            let lowest = bucket_keys(buckets, min_freq);
            let victim = num(lowest[lowest.len() - 1]);
            step("victim", `Bucket ${min_freq} is the least-used bucket; its *last* entry, ${victim}, is the least recently used key in it. Both tie-breaks answered by one lookup.`,
                [ kv(global::VALUES, vals).bad([victim]), kv(global::FREQ, freq).bad([victim]),
                  text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);

            lowest.pop();
            buckets = bucket_put(buckets, min_freq, lowest);
            map_del(vals, victim);
            map_del(freq, victim);
            log(`evict ${victim}`);
            step("evict", `Evict ${victim} from every structure at once — a key left behind in one map is the classic LFU bug.`,
                [ kv(global::VALUES, vals), kv(global::FREQ, freq),
                  text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);
        }

        map_set(vals, key, value);
        map_set(freq, key, 1);
        let ones = bucket_keys(buckets, 1);
        ones.insert(0, `${key}`);
        buckets = bucket_put(buckets, 1, ones);
        step("insert", `Insert ${key} → ${value} with use count 1, at the front of bucket 1.`,
            [ kv(global::VALUES, vals).hl([key]), kv(global::FREQ, freq).hl([key]),
              text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ]);

        min_freq = 1;
        set(#{ minFreq: N(1), size: N(map_len(vals)) });
        step("newmin", "minFreq drops back to 1 — a brand-new key is by definition the least used one.",
            [ text("buckets", bucket_lines(buckets, min_freq, map_len(vals), cap)) ], "return");
        leave("");
    }

    let answer = join(out, " ");
    log(answer, "result");
    result(answer);
}
