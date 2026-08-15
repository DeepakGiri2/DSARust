// Valid Anagram — count up with one string, count down with the other.
//
// The counter map is the whole idea: if the two strings use the same letters
// the same number of times, every count returns to zero and none ever goes
// negative on the way.

fn trace(input) {
    let s = chars(input.s);
    let t = chars(input.t);
    let count = map_new();

    enter("isAnagram", #{ s: S(input.s), t: S(input.t) });

    if s.len() != t.len() {
        step("len", `s has ${s.len()} character(s) and t has ${t.len()} — different lengths.`,
            [ array("s", s), array("t", t) ]);
        log("lengths differ", "result");
        result("false");
        step("lenbad", "Strings of different lengths cannot be anagrams, so stop here.",
            [ array("s", s), array("t", t) ], "return");
        leave("false");
        return;
    }

    step("len", `Both strings have ${s.len()} characters, so an anagram is still possible.`,
        [ array("s", s), array("t", t) ]);

    set(#{ count: MAP(count) });
    step("init", "One counter per letter. Nothing counted yet.",
        [ array("s", s), array("t", t), kv("count", count) ]);

    for i in 0..s.len() {
        let c = s[i];
        step("builds", `Take '${c}' from s.`,
            [ array("s", s).ptr("i", i).done(before(i)), array("t", t), kv("count", count) ]);
        let n = map_inc(count, c, 1);
        set(#{ count: MAP(count, [c]) });
        log(`count['${c}'] = ${n}`);
        step("inc", `count['${c}'] is now ${n}.`,
            [ array("s", s).ptr("i", i).hl([i]).done(before(i)), array("t", t),
              kv("count", count).hl([c]) ]);
    }

    for j in 0..t.len() {
        let c = t[j];
        step("scant", `Now spend '${c}' from t against the tally.`,
            [ array("s", s).done(seq(0, s.len() - 1)), array("t", t).ptr("j", j).done(before(j)),
              kv("count", count) ]);

        let n = map_inc(count, c, -1);
        set(#{ count: MAP(count, [c]) });
        log(`count['${c}'] = ${n}`);
        step("dec", `count['${c}'] drops to ${n}.`,
            [ array("s", s).done(seq(0, s.len() - 1)), array("t", t).ptr("j", j).hl([j]).done(before(j)),
              kv("count", count).hl([c]) ]);

        if n < 0 {
            log(`'${c}' appears more often in t than in s`, "result");
            result("false");
            step("check", `count['${c}'] went below zero: t uses '${c}' more often than s does.`,
                [ array("t", t).ptr("j", j).bad([j]), kv("count", count).bad([c]) ]);
            step("bad", "Return false.",
                [ array("t", t).bad([j]), kv("count", count).bad([c]) ], "return");
            leave("false");
            return;
        }
        step("check", `count['${c}'] is ${n}, still not negative — keep going.`,
            [ array("s", s).done(seq(0, s.len() - 1)), array("t", t).ptr("j", j).done(before(j)),
              kv("count", count) ]);
    }

    log("every count returned to zero", "result");
    result("true");
    step("ok",
        "Equal lengths and no count ever went negative, so every letter was used exactly as often in both. They are anagrams.",
        [ array("s", s).done(seq(0, s.len() - 1)), array("t", t).done(seq(0, t.len() - 1)),
          kv("count", count) ],
        "return");
    leave("true");
}
