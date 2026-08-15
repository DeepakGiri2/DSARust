// Insert Delete GetRandom — swap with the last element instead of shifting.
//
// Two requirements pull in opposite directions: a uniform random pick needs a
// dense array, while O(1) deletion needs a hash map. Keeping both, with the
// map storing each value's array index, satisfies each of them — as long as
// deletion never shifts the array. Overwriting the doomed slot with the last
// element and shrinking does that in two moves, and the only casualty is the
// insertion order, which nothing here ever used.
//
// getRandom takes its index as an argument here so the animation is
// reproducible; the real thing draws it at random.

fn trace(input) {
    let idx = map_new();
    let vals = [];
    let out = [];

    enter("RandomizedSet", #{});
    step("init", "The map says where each value lives in the array.",
        [ kv("value → index", idx) ]);
    step("vals", "The array is kept dense so that any index is a valid draw.",
        [ array("values", vals) ]);

    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() < 2 { continue; }
        let v = num(parts[1]);

        if parts[0] == "insert" {
            enter(`insert(${v})`, #{ v: N(v) });
            if map_has(idx, v) {
                out.push("false");
                log(`insert(${v}) -> false`, "result");
                step("hasInsert", `${v} is already in the set.`,
                    [ kv("value → index", idx).hl([`${v}`]), text("output", out) ], "return");
                leave("false");
                continue;
            }
            step("hasInsert", `${v} is not present yet.`,
                [ kv("value → index", idx) ]);

            map_set(idx, v, vals.len());
            step("record", `Record that ${v} will sit at index ${vals.len()}.`,
                [ kv("value → index", idx).hl([`${v}`]) ]);

            vals.push(v);
            out.push("true");
            log(`insert ${v} at ${vals.len() - 1}`, "result");
            step("append", `Append it to the array.`,
                [ array("values", vals).hl([vals.len() - 1]) ]);
            step("inserted", "Inserted.", [ text("output", out) ], "return");
            leave("true");
            continue;
        }

        if parts[0] == "remove" {
            enter(`remove(${v})`, #{ v: N(v) });
            step("lookup", `Where is ${v}?`, [ kv("value → index", idx) ]);

            if !map_has(idx, v) {
                out.push("false");
                log(`remove(${v}) -> false`, "result");
                step("absent", `${v} is not in the set.`, [ text("output", out) ], "return");
                leave("false");
                continue;
            }
            let i = map_get(idx, v);
            step("absent", `${v} is at index ${i}.`,
                [ array("values", vals).ptr("i", i).hl([i]),
                  kv("value → index", idx).hl([`${v}`]) ]);

            let last = vals[vals.len() - 1];
            step("last", `The last element is ${last}. Removing from the middle by shifting would be linear, so it is not done.`,
                [ array("values", vals).pointers(#{ i: i, last: vals.len() - 1 })
                    .hl([vals.len() - 1]) ]);

            vals[i] = last;
            step("move", `Overwrite index ${i} with ${last}. That destroys the ordering, which nothing here relies on.`,
                [ array("values", vals).ptr("i", i).hl([i]) ]);

            map_set(idx, last, i);
            step("reindex", `${last} has moved, so its recorded index becomes ${i} — forgetting this is the classic bug.`,
                [ kv("value → index", idx).hl([`${last}`]) ]);

            vals.truncate(vals.len() - 1);
            step("shrink", "Drop the duplicate copy at the end.",
                [ array("values", vals) ]);

            map_del(idx, v);
            out.push("true");
            log(`remove ${v}`, "result");
            step("forget", `And forget ${v}.`, [ kv("value → index", idx) ]);
            step("removed", "Removed.", [ text("output", out) ], "return");
            leave("true");
            continue;
        }

        enter(`getRandom()`, #{});
        let pick = v % vals.len();
        out.push(`${vals[pick]}`);
        log(`getRandom -> ${vals[pick]}`, "result");
        step("random", `Draw index ${pick}: ${vals[pick]}. Every index is equally likely precisely because the array has no gaps.`,
            [ array("values", vals).ptr("pick", pick).hl([pick]), text("output", out) ], "return");
        leave(`${vals[pick]}`);
    }

    let answer = join(out, " ");
    result(answer);
    leave(answer);
}
