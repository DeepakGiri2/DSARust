// Isomorphic Strings — one map is a trap; you need both directions.
//
// s → t alone happily accepts "badc" → "baba": b→b, a→a, d→b, c→a. Every
// character of s maps consistently — but two of them collide onto the same
// character of t, which the problem forbids. The reverse map is what catches
// it, and it is the case interviewers reach for first.

const FWD = "s → t";
const BACK = "t → s";

fn trace(input) {
    let s = chars(input.s);
    let t = chars(input.t);
    let n = s.len();

    enter("isIsomorphic", #{ s: S(input.s), t: S(input.t) });

    let st = map_new();
    let ts = map_new();
    step("init", "Two maps, one per direction. Together they force the correspondence to be one-to-one both ways.",
        [ array("s", s), array("t", t), kv(global::FWD, st), kv(global::BACK, ts) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `Position ${i}.`,
            [ array("s", s).ptr("i", i).done(before(i)),
              array("t", t).ptr("i", i).done(before(i)) ]);

        let a = s[i];
        let b = t[i];
        set(#{ a: S(a), b: S(b) });
        step("pair", `${a} must correspond to ${b}.`,
            [ array("s", s).ptr("i", i).hl([i]), array("t", t).ptr("i", i).hl([i]),
              kv(global::FWD, st), kv(global::BACK, ts) ]);

        if map_has(st, a) && map_get(st, a) != b {
            log(`${a} already maps to ${map_get(st, a)}`, "result");
            step("fwd", `${a} was already mapped to ${map_get(st, a)}, not ${b} — the same character of s cannot become two different characters.`,
                [ kv(global::FWD, st).bad([a]) ]);
            result("false");
            step("no", "Return false.", [ kv(global::FWD, st).bad([a]) ], "return");
            leave("false");
            return;
        }
        step("fwd", `Forward direction is fine: ${a} ${if map_has(st, a) { `already maps to ${b}` } else { "has no mapping yet" }}.`,
            [ kv(global::FWD, st) ]);

        if map_has(ts, b) && map_get(ts, b) != a {
            log(`${b} is already taken by ${map_get(ts, b)}`, "result");
            step("back", `But ${b} is already spoken for by ${map_get(ts, b)}. Two characters of s would collide onto ${b} — this is exactly what a single map misses.`,
                [ kv(global::BACK, ts).bad([b]) ]);
            result("false");
            step("no2", "Return false.", [ kv(global::BACK, ts).bad([b]) ], "return");
            leave("false");
            return;
        }
        step("back", `And ${b} is not claimed by any other character.`,
            [ kv(global::BACK, ts) ]);

        map_set(st, a, b);
        step("map1", `Record ${a} → ${b}.`, [ kv(global::FWD, st).hl([a]) ]);
        map_set(ts, b, a);
        step("map2", `Record ${b} → ${a}.`, [ kv(global::BACK, ts).hl([b]) ]);
    }

    log("isomorphic", "result");
    result("true");
    step("yes", "Every position agreed in both directions — the strings are isomorphic.",
        [ kv(global::FWD, st), kv(global::BACK, ts) ], "return");
    leave("true");
}
