// Is Subsequence — one pointer per string, and only one ever rewinds.
//
// Greedy matching is safe here: taking the *earliest* possible match for each
// character of s never rules out a later solution, so there is no need to try
// alternatives.

fn trace(input) {
    let s = chars(input.s);
    let t = chars(input.t);
    let i = 0;
    let j = 0;
    let matched = [];

    enter("isSubsequence", #{ s: S(input.s), t: S(input.t) });
    set(#{ i: N(i), j: N(j) });
    step("init", "i walks the needle s; j walks the haystack t. j never goes backwards.",
        [ array("s (needle)", s).ptr("i", i), array("t (haystack)", t).ptr("j", j) ]);

    while i < s.len() && j < t.len() {
        step("loop", `Looking for '${s[i]}' from position ${j} of t.`,
            [ array("s (needle)", s).ptr("i", i).done(before(i)),
              array("t (haystack)", t).ptr("j", j).hl(matched).done(before(j)) ]);

        if s[i] == t[j] {
            matched.push(j);
            step("match", `'${t[j]}' matches — take the earliest copy, which can never cost us a later match.`,
                [ array("s (needle)", s).ptr("i", i).hl([i]),
                  array("t (haystack)", t).ptr("j", j).hl(matched) ]);
            i += 1;
            set(#{ i: N(i) });
            step("advI", `i = ${i}: ${i} of ${s.len()} matched.`,
                [ array("s (needle)", s).ptr("i", i).done(before(i)),
                  array("t (haystack)", t).ptr("j", j).hl(matched) ]);
        } else {
            step("match", `'${t[j]}' is not '${s[i]}' — skip it.`,
                [ array("s (needle)", s).ptr("i", i),
                  array("t (haystack)", t).ptr("j", j).bad([j]).hl(matched) ]);
        }

        j += 1;
        set(#{ j: N(j) });
        step("advJ", `j = ${j}.`,
            [ array("s (needle)", s).ptr("i", i).done(before(i)),
              array("t (haystack)", t).ptr("j", j).hl(matched).done(before(j)) ]);
    }

    let ok = i == s.len();
    log(`matched ${i} of ${s.len()}`, "result");
    result(`${ok}`);
    step("ret",
        if ok {
            `Every character of s was matched in order, so it is a subsequence.`.to_string()
        } else {
            `t ran out with only ${i} of ${s.len()} characters matched, so s is not a subsequence.`
        },
        [ array("s (needle)", s).hl(if ok { seq(0, s.len() - 1) } else { before(i) }),
          array("t (haystack)", t).hl(matched) ],
        "return");
    leave(`${ok}`);
}
