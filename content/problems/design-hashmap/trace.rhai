// Design HashMap — a hash set whose entries carry a value.
//
// Structurally identical to the set: hash to a bucket, scan that bucket. The
// difference is what happens on a hit. A set can stop at "already there"; a
// map has to reach into the entry and overwrite its value, which is why the
// put scan looks for the entry itself rather than just a yes or no.

fn bucket_view(buckets) {
    let out = [];
    for b in 0..buckets.len() {
        let cells = [];
        for e in buckets[b] { cells.push(`${e[0]}=${e[1]}`); }
        out.push(`bucket ${b}: ${if cells.len() == 0 { "-" } else { join(cells, ", ") }}`);
    }
    out
}

fn trace(input) {
    let n_buckets = 8;
    let buckets = [];
    for b in 0..n_buckets { buckets.push([]); }
    let out = [];

    enter("MyHashMap", #{ buckets: N(n_buckets) });
    step("init", `${n_buckets} buckets, each holding key-value entries.`,
        [ text("buckets", bucket_view(buckets)) ]);
    step("hash", `Slot = key mod ${n_buckets}.`, [ text("buckets", bucket_view(buckets)) ]);

    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() < 2 { continue; }
        let key = num(parts[1]);
        let b = key % n_buckets;

        if parts[0] == "put" {
            let value = num(parts[2]);
            enter(`put(${key}, ${value})`, #{ key: N(key), value: N(value) });
            step("putHash", `${key} hashes to bucket ${b}.`,
                [ text("buckets", bucket_view(buckets)).hl([b]) ]);

            let at = -1;
            for i in 0..buckets[b].len() { if buckets[b][i][0] == key { at = i; } }
            step("scanPut", `Bucket ${b}: ${if buckets[b].len() == 0 { "empty" } else { join(bucket_keys(buckets[b]), ", ") }}.`,
                [ text("buckets", bucket_view(buckets)).hl([b]) ]);

            if at >= 0 {
                step("existing", `${key} is already stored, holding ${buckets[b][at][1]}.`,
                    [ text("buckets", bucket_view(buckets)).hl([b]) ]);
                buckets[b][at][1] = value;
                log(`update ${key} -> ${value}`);
                step("update", `Overwrite it with ${value} — the difference between a map and a set.`,
                    [ text("buckets", bucket_view(buckets)).hl([b]) ], "return");
            } else {
                buckets[b].push([key, value]);
                log(`put ${key} -> ${value} in bucket ${b}`);
                step("append", `New key: append ${key}=${value}.`,
                    [ text("buckets", bucket_view(buckets)).hl([b]) ], "return");
            }
            leave("");
            continue;
        }

        if parts[0] == "get" {
            enter(`get(${key})`, #{ key: N(key) });
            step("getHash", `${key} hashes to bucket ${b}.`,
                [ text("buckets", bucket_view(buckets)).hl([b]) ]);
            step("scanGet", `Scan bucket ${b} only.`,
                [ text("buckets", bucket_view(buckets)).hl([b]) ]);

            let found = -1;
            for e in buckets[b] { if e[0] == key { found = e[1]; } }
            out.push(`${found}`);
            log(`get(${key}) -> ${found}`, "result");
            if found >= 0 {
                step("hit", `${key} maps to ${found}.`, [ text("output", out) ], "return");
            } else {
                step("miss", `${key} is not stored, so -1.`, [ text("output", out) ], "return");
            }
            leave(`${found}`);
            continue;
        }

        enter(`remove(${key})`, #{ key: N(key) });
        step("removeHash", `${key} hashes to bucket ${b}.`,
            [ text("buckets", bucket_view(buckets)).hl([b]) ]);
        step("scanRemove", `Scan bucket ${b}.`,
            [ text("buckets", bucket_view(buckets)).hl([b]) ]);

        let kept = [];
        let hit = false;
        for e in buckets[b] {
            if e[0] == key && !hit { hit = true; } else { kept.push(e); }
        }
        if hit {
            step("found", `${key} is here.`, [ text("buckets", bucket_view(buckets)).hl([b]) ]);
            buckets[b] = kept;
            log(`remove ${key}`);
            step("unlink", "Drop the entry.",
                [ text("buckets", bucket_view(buckets)).hl([b]) ], "return");
        } else {
            step("found", `${key} is not stored, so nothing to remove.`,
                [ text("buckets", bucket_view(buckets)).hl([b]) ], "return");
        }
        leave("");
    }

    let answer = join(out, " ");
    result(answer);
    leave(answer);
}

fn bucket_keys(bucket) {
    let out = [];
    for e in bucket { out.push(`${e[0]}=${e[1]}`); }
    out
}
