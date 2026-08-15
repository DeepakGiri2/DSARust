// LRU Cache — a map for "where", a list for "when".
//
// Neither structure alone can do it. A hash map answers "is this key here?"
// instantly but has no notion of order; a doubly linked list keeps perfect
// order and can splice any node out in constant time, but finding that node
// means walking. Making the map's value the list *node* removes the walk, and
// that is the whole design: every operation becomes a lookup plus two pointer
// rewrites.
//
// The recency strip below is that linked list; most recently used is leftmost.

fn keys_of(order, store) {
    let out = [];
    for k in order { out.push(`${k}=${map_get(store, k)}`); }
    out
}

fn touch(order, key) {
    let out = [key];
    for k in order {
        if k != key { out.push(k); }
    }
    out
}

fn trace(input) {
    let cap = input.capacity;
    let store = map_new();
    let order = [];
    let out = [];

    enter("LRUCache", #{ capacity: N(cap) });
    step("init", `Capacity ${cap}. The map finds a key; the list remembers how recently each one was used.`,
        [ kv("map: key → node", store), array("recency (most recent first)", keys_of(order, store)) ]);

    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() == 0 { continue; }

        if parts[0] == "get" {
            let key = num(parts[1]);
            enter(`get(${key})`, #{ key: N(key) });

            let hit = map_has(store, key);
            step("lookup", `Look ${key} up in the map — a single hash lookup, no walking.`,
                [ kv("map: key → node", store).hl(if hit { [`${key}`] } else { [] }),
                  array("recency (most recent first)", keys_of(order, store)) ]);

            if !hit {
                out.push("-1");
                log(`get(${key}) -> miss`, "result");
                step("miss", `${key} is not cached.`,
                    [ kv("map: key → node", store), text("output", out) ]);
                step("retMiss", "Return -1, and do not disturb the recency order.",
                    [ array("recency (most recent first)", keys_of(order, store)), text("output", out) ],
                    "return");
                leave("-1");
                continue;
            }

            let v = map_get(store, key);
            order = touch(order, key);
            log(`get(${key}) -> ${v}`, "result");
            step("touch", `Found. Reading counts as using, so ${key} moves to the front — unlink and re-insert, both O(1) because the node itself was in the map.`,
                [ kv("map: key → node", store).hl([`${key}`]),
                  array("recency (most recent first)", keys_of(order, store)).hl([0]) ]);
            out.push(`${v}`);
            step("retHit", `Return ${v}.`,
                [ array("recency (most recent first)", keys_of(order, store)), text("output", out) ],
                "return");
            leave(`${v}`);
            continue;
        }

        let key = num(parts[1]);
        let value = num(parts[2]);
        enter(`put(${key}, ${value})`, #{ key: N(key), value: N(value) });

        if map_has(store, key) {
            step("exists", `${key} is already cached, so nothing is evicted.`,
                [ kv("map: key → node", store).hl([`${key}`]),
                  array("recency (most recent first)", keys_of(order, store)) ]);
            map_set(store, key, value);
            step("update", `Overwrite its value with ${value}.`,
                [ kv("map: key → node", store).hl([`${key}`]) ]);
            order = touch(order, key);
            log(`put(${key}, ${value}) — refreshed`);
            step("touchPut", "Writing counts as using too, so it moves to the front.",
                [ array("recency (most recent first)", keys_of(order, store)).hl([0]) ], "return");
            leave("");
            continue;
        }

        if map_len(store) == cap {
            step("full", `The cache already holds ${cap} key(s), so something has to go.`,
                [ kv("map: key → node", store),
                  array("recency (most recent first)", keys_of(order, store)) ]);
            let victim = order[order.len() - 1];
            step("victim", `The last entry, ${victim}, is the least recently used — the list keeps that answer at its tail for free.`,
                [ array("recency (most recent first)", keys_of(order, store)).bad([order.len() - 1]) ]);
            order.truncate(order.len() - 1);
            step("unlink", `Unlink it from the recency list.`,
                [ array("recency (most recent first)", keys_of(order, store)) ]);
            map_del(store, victim);
            log(`evict ${victim}`);
            step("evict", `Remove ${victim} from the map too, or the map would keep growing.`,
                [ kv("map: key → node", store) ]);
        } else {
            step("full", `The cache holds ${map_len(store)} of ${cap} key(s), so there is room.`,
                [ kv("map: key → node", store),
                  array("recency (most recent first)", keys_of(order, store)) ]);
        }

        step("newNode", `Make a node for ${key} = ${value}.`,
            [ kv("map: key → node", store) ]);
        map_set(store, key, value);
        step("insert", `Record it in the map so it can be found again in O(1).`,
            [ kv("map: key → node", store).hl([`${key}`]) ]);
        order.insert(0, key);
        log(`put(${key}, ${value})`);
        step("front", `Push it to the front of the recency list — a brand new entry is by definition the most recently used.`,
            [ array("recency (most recent first)", keys_of(order, store)).hl([0]) ], "return");
        leave("");
    }

    let answer = join(out, " ");
    result(answer);
    leave(answer);
}
