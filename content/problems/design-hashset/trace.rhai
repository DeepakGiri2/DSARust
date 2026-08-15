// Buckets rendered as one text line each, so a collision is visible as a
// bucket holding more than one entry.
fn bucket_view(buckets) {
    let out = [];
    for b in 0..buckets.len() {
        out.push(`bucket ${b}: ${if buckets[b].len() == 0 { "-" } else { join(buckets[b], ", ") }}`);
    }
    out
}

// Design HashSet — the key computes its own address.
//
// The whole idea is that no searching is needed to find *where* a key lives:
// the hash sends it straight to one slot. Two keys can collide there, so each
// slot keeps a short list, and every operation is "hash, then scan that one
// list". The cost is constant as long as the lists stay short — which is what
// a good hash function and enough buckets buy you.

fn trace(input) {
    let n_buckets = 8;
    let buckets = [];
    for b in 0..n_buckets { buckets.push([]); }
    let out = [];

    enter("MyHashSet", #{ buckets: N(n_buckets) });
    step("init", `${n_buckets} buckets, each a short list of keys that hash to it.`,
        [ text("buckets", bucket_view(buckets)) ]);
    step("hash", `The hash is simply key mod ${n_buckets}, so a key's slot is arithmetic rather than a search.`,
        [ text("buckets", bucket_view(buckets)) ]);

    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() < 2 { continue; }
        let key = num(parts[1]);
        let b = key % n_buckets;

        if parts[0] == "add" {
            enter(`add(${key})`, #{ key: N(key) });
            step("addHash", `${key} mod ${n_buckets} = ${b}, so it belongs in bucket ${b}.`,
                [ text("buckets", bucket_view(buckets)).hl([b]) ]);

            let present = false;
            for v in buckets[b] { if v == key { present = true; } }
            step("scanAdd", `Bucket ${b} holds ${if buckets[b].len() == 0 { "nothing" } else { join(buckets[b], ", ") }}. Only this one list has to be scanned.`,
                [ text("buckets", bucket_view(buckets)).hl([b]) ]);

            if present {
                step("present", `${key} is already there — a set holds no duplicates.`,
                    [ text("buckets", bucket_view(buckets)).hl([b]) ], "return");
                leave("");
                continue;
            }
            buckets[b].push(key);
            log(`add ${key} to bucket ${b}`);
            step("append", `Append ${key}. Bucket ${b} now holds ${plural(buckets[b].len(), "key")}${if buckets[b].len() > 1 { " — a collision" } else { "" }}.`,
                [ text("buckets", bucket_view(buckets)).hl([b]) ], "return");
            leave("");
            continue;
        }

        if parts[0] == "remove" {
            enter(`remove(${key})`, #{ key: N(key) });
            step("removeHash", `${key} hashes to bucket ${b}.`,
                [ text("buckets", bucket_view(buckets)).hl([b]) ]);
            step("scanRemove", `Scan bucket ${b}: ${if buckets[b].len() == 0 { "empty" } else { join(buckets[b], ", ") }}.`,
                [ text("buckets", bucket_view(buckets)).hl([b]) ]);

            let kept = [];
            let hit = false;
            for v in buckets[b] {
                if v == key && !hit { hit = true; } else { kept.push(v); }
            }
            if hit {
                step("found", `${key} is in the bucket.`,
                    [ text("buckets", bucket_view(buckets)).hl([b]) ]);
                buckets[b] = kept;
                log(`remove ${key}`);
                step("unlink", "Drop it from the list.",
                    [ text("buckets", bucket_view(buckets)).hl([b]) ], "return");
            } else {
                step("found", `${key} is not in the bucket, so there is nothing to remove.`,
                    [ text("buckets", bucket_view(buckets)).hl([b]) ], "return");
            }
            leave("");
            continue;
        }

        enter(`contains(${key})`, #{ key: N(key) });
        step("containsHash", `${key} hashes to bucket ${b}.`,
            [ text("buckets", bucket_view(buckets)).hl([b]) ]);
        step("scanContains", `Scan bucket ${b}: ${if buckets[b].len() == 0 { "empty" } else { join(buckets[b], ", ") }}. Every other bucket is irrelevant.`,
            [ text("buckets", bucket_view(buckets)).hl([b]) ]);

        let hit = false;
        for v in buckets[b] { if v == key { hit = true; } }
        out.push(`${hit}`);
        log(`contains(${key}) -> ${hit}`, "result");
        if hit {
            step("hit", `Found ${key}.`, [ text("output", out) ], "return");
        } else {
            step("miss", `${key} is not present.`, [ text("output", out) ], "return");
        }
        leave(`${hit}`);
    }

    let answer = join(out, " ");
    result(answer);
    leave(answer);
}
