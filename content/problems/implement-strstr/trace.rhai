// KMP — the haystack pointer never goes backwards.
//
// That single property is the whole algorithm. The naive scan, on a mismatch
// after k matched characters, throws those k away and re-reads them. KMP knows
// something the naive scan does not: those k characters are the needle's own
// prefix, so any *shorter* prefix that is also a suffix of them is already
// matched. lps[j-1] is exactly how long that is, so j slides back and i stays
// put. Watch i in the second half — it only ever increases.

const HAY = "haystack";
const NEEDLE = "needle";
const LPS = "lps — longest prefix that is also a suffix";

fn trace(input) {
    let hay = chars(input.haystack);
    let need = chars(input.needle);
    let n = hay.len();
    let m = need.len();

    enter("strStr", #{ haystack: S(input.haystack), needle: S(input.needle) });

    let lps = [];
    for i in 0..m { lps.push(0); }
    set(#{ lps: A(lps) });
    step("lps", `Phase 1: build the table. lps[i] answers "if the match dies at needle[${"i"}+1], how much of the needle is still matched?" — before ever looking at the haystack.`,
        [ array(global::NEEDLE, need), array(global::LPS, lps) ]);

    let i = 1;
    let length = 0;
    while i < m {
        set(#{ i: N(i), length: N(length) });
        step("build", `i = ${i}, length = ${length}: compare needle[${i}] = ${need[i]} with needle[${length}] = ${need[length]}.`,
            [ array(global::NEEDLE, need).ptr("i", i).ptr("len", length).done(before(i)),
              array(global::LPS, lps).done(before(i)) ]);

        if need[i] == need[length] {
            length += 1;
            step("match", `They agree, so the prefix that is also a suffix just grew to ${length}.`,
                [ array(global::NEEDLE, need).ptr("i", i).ptr("len", length).hl([i, length - 1]) ]);
            lps[i] = length;
            set(#{ lps: A(lps), length: N(length) });
            step("set", `lps[${i}] = ${length}.`,
                [ array(global::NEEDLE, need).ptr("i", i).done(before(i)),
                  array(global::LPS, lps).hl([i]) ]);
            i += 1;
        } else if length > 0 {
            step("fall", `${need[i]} ≠ ${need[length]}. Rather than give up on the whole prefix, fall back to the next-longest one.`,
                [ array(global::NEEDLE, need).ptr("i", i).ptr("len", length).bad([i]) ]);
            length = lps[length - 1];
            set(#{ length: N(length) });
            step("back", `length = lps[${"..."}] = ${length} — the table is consulting itself. Note i did not move.`,
                [ array(global::NEEDLE, need).ptr("i", i).ptr("len", length),
                  array(global::LPS, lps) ]);
        } else {
            lps[i] = 0;
            set(#{ lps: A(lps) });
            step("zero", `No prefix of any length survives here, so lps[${i}] = 0.`,
                [ array(global::LPS, lps).hl([i]) ]);
            i += 1;
        }
    }

    unset("length");
    log(`lps = ${brackets(lps)}`);
    step("scan", `Table done: ${brackets(lps)}. Phase 2: one walk over the haystack, and i will never move backwards.`,
        [ array(global::HAY, hay), array(global::NEEDLE, need), array(global::LPS, lps) ]);

    let hi = 0;
    let j = 0;
    while hi < n {
        set(#{ i: N(hi), j: N(j) });
        step("cmp", `haystack[${hi}] = ${hay[hi]} vs needle[${j}] = ${need[j]}.`,
            [ array(global::HAY, hay).ptr("i", hi).window(hi - j, hi - j + m - 1).done(before(hi - j)),
              array(global::NEEDLE, need).ptr("j", j).done(before(j)) ]);

        if hay[hi] == need[j] {
            hi += 1;
            j += 1;
            set(#{ i: N(hi), j: N(j) });
            step("adv", `Match — ${j} of ${m} needle characters are lined up.`,
                [ array(global::HAY, hay).ptr("i", hi).hl(span(hi - j, hi - 1)),
                  array(global::NEEDLE, need).ptr("j", j).done(before(j)) ]);

            if j == m {
                let at = hi - j;
                log(`found at ${at}`, "result");
                step("done", `All ${m} matched.`,
                    [ array(global::HAY, hay).hl(span(at, hi - 1)), array(global::NEEDLE, need).done(seq(0, m - 1)) ]);
                result(`${at}`);
                step("found", `The needle starts at index ${hi} − ${j} = ${at}.`,
                    [ array(global::HAY, hay).hl(span(at, hi - 1)).done(before(at)) ], "return");
                leave(`${at}`);
                return;
            }
        } else if j > 0 {
            step("shift", `Mismatch after ${j} matched characters — and this is where KMP earns its keep.`,
                [ array(global::HAY, hay).ptr("i", hi).bad([hi]).hl(span(hi - j, hi - 1)),
                  array(global::NEEDLE, need).ptr("j", j).bad([j]) ]);
            j = lps[j - 1];
            set(#{ j: N(j) });
            step("jump", `Slide the needle so ${j} characters are still matched. i stays at ${hi}: nothing is re-read.`,
                [ array(global::HAY, hay).ptr("i", hi).hl(span(hi - j, hi - 1)),
                  array(global::NEEDLE, need).ptr("j", j).done(before(j)),
                  array(global::LPS, lps).hl([j]) ]);
        } else {
            step("next", `Mismatch on the needle's very first character — there is nothing to salvage, so just move on.`,
                [ array(global::HAY, hay).ptr("i", hi).bad([hi]).done(before(hi)) ]);
            hi += 1;
            set(#{ i: N(hi) });
        }
    }

    log("not found", "result");
    result("-1");
    step("none", "The haystack ran out with no full match — return -1.",
        [ array(global::HAY, hay).done(seq(0, n - 1)) ], "return");
    leave("-1");
}
