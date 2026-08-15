// Permutation in String — a fixed window compared against a letter count.
//
// A permutation is exactly "the same letters, the same number of times", so
// order never has to be considered. Slide a window of len(s1) and compare two
// small tallies.

fn counts_of(cs) {
    let m = map_new();
    for c in cs { map_inc(m, c, 1); }
    m
}

fn same(a, b) {
    if map_len(a) != map_len(b) { return false; }
    for k in map_keys(a) {
        if map_get(a, k) != map_get(b, k) { return false; }
    }
    true
}

fn trace(input) {
    let s1 = chars(input.s1);
    let s2 = chars(input.s2);
    let n = s1.len();

    enter("checkInclusion", #{ s1: S(input.s1), s2: S(input.s2) });
    step("n", `s1 has ${n} letter(s), so only windows of exactly that width can be permutations.`,
        [ array("s1", s1), array("s2", s2) ]);

    if n > s2.len() {
        step("guard", "s1 is longer than s2 — no window can hold it.", [ array("s2", s2) ]);
        result("false");
        step("guardRet", "Return false.", [ array("s2", s2) ], "return");
        leave("false");
        return;
    }
    step("guard", "s2 is long enough to contain a window.", [ array("s2", s2) ]);

    let need = counts_of(s1);
    set(#{ need: MAP(need) });
    step("need", `The letters s1 needs: ${map_len(need)} distinct.`,
        [ array("s1", s1), kv("need", need) ]);

    let win = map_new();
    step("win", "The window's own tally starts empty.", [ array("s2", s2), kv("window", win) ]);

    for i in 0..n {
        map_inc(win, s2[i], 1);
        step("build", `Build the first window: take '${s2[i]}'.`,
            [ array("s2", s2).window(0, i).hl([i]), kv("need", need), kv("window", win) ]);
    }
    set(#{ window: MAP(win) });
    step("bn", `The first window is "${join(s2.extract(0, n), "")}".`,
        [ array("s2", s2).window(0, n - 1), kv("need", need), kv("window", win) ]);
    step("bw", "Now it only ever changes by one letter in and one letter out.",
        [ array("s2", s2).window(0, n - 1), kv("window", win) ]);

    let right = n - 1;
    while right < s2.len() {
        let left = right - n + 1;
        set(#{ right: N(right), left: N(left) });
        step("loop", `Window [${left}..${right}] = "${join(s2.extract(left, n), "")}".`,
            [ array("s2", s2).pointers(#{ left: left, right: right })
                .window(left, right).done(before(left)),
              kv("need", need), kv("window", win) ]);

        if right >= n {
            step("slide", `Slide: '${s2[right]}' enters, '${s2[left - 1]}' leaves.`,
                [ array("s2", s2).window(left, right).hl([right]).bad([left - 1]),
                  kv("window", win) ]);
            step("addR", `count['${s2[right]}'] goes up.`,
                [ array("s2", s2).window(left, right).hl([right]), kv("window", win).hl([s2[right]]) ]);
            step("subL", `count['${s2[left - 1]}'] goes down.`,
                [ array("s2", s2).window(left, right).bad([left - 1]),
                  kv("window", win).hl([s2[left - 1]]) ]);
        }

        step("ok", "Assume the window matches until a letter disagrees.",
            [ array("s2", s2).window(left, right), kv("need", need), kv("window", win) ]);

        let matched = same(need, win);
        step("compare", "Compare the two tallies letter by letter.",
            [ array("s2", s2).window(left, right), kv("need", need), kv("window", win) ]);

        if !matched {
            step("diff", "At least one letter appears a different number of times.",
                [ array("s2", s2).window(left, right).bad(span(left, right)),
                  kv("need", need), kv("window", win) ]);
            step("setno", "So this window is not a permutation.",
                [ array("s2", s2).window(left, right), kv("window", win) ]);
            step("brk", "Stop comparing and slide on.",
                [ array("s2", s2).window(left, right) ]);
        } else {
            log(`window [${left}..${right}] matches`, "result");
            step("match", "Every letter appears exactly as often as s1 needs.",
                [ array("s2", s2).hl(span(left, right)), kv("need", need), kv("window", win) ]);
            result("true");
            step("found", `"${join(s2.extract(left, n), "")}" is a permutation of "${input.s1}". Return true.`,
                [ array("s2", s2).hl(span(left, right)) ], "return");
            leave("true");
            return;
        }

        // Advance the window before the next round.
        if right + 1 < s2.len() {
            map_inc(win, s2[right + 1], 1);
            map_inc(win, s2[left], -1);
            if map_get(win, s2[left]) == 0 { map_del(win, s2[left]); }
            set(#{ window: MAP(win) });
        }
        right += 1;
    }

    log("no window matched", "result");
    result("false");
    step("ret", "No window of the right width had the right letters — s2 contains no permutation of s1.",
        [ array("s2", s2).done(seq(0, s2.len() - 1)) ], "return");
    leave("false");
}
