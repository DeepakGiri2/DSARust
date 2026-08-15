// Longest Substring Without Repeating Characters — a window that never
// travels backwards.
//
// `left` only ever moves forward, and it jumps straight past the previous
// occurrence of a repeat rather than sliding one step at a time. That jump is
// what keeps this linear, and it is the step worth pausing on.

fn trace(input) {
    let s = chars(input.s);
    let n = s.len();
    let last = map_new();
    let best = 0;
    let best_lo = 0;
    let left = 0;

    enter("lengthOfLongestSubstring", #{ s: S(input.s) });
    set(#{ last: MAP(last) });
    step("init", "last[c] will remember the most recent index of every character.",
        [ array("s", s), kv("last — char → index", last) ]);

    set(#{ best: N(best), left: N(left) });
    step("init2", "The window is empty and the best answer so far is 0.",
        [ array("s", s).pointers(#{ left: left }), kv("last — char → index", last) ]);

    for right in 0..n {
        let c = s[right];
        set(#{ right: N(right), c: S(c) });
        step("loop", `right = ${right}: the character is '${c}'.`,
            [ array("s", s).pointers(#{ left: left, right: right }).window(left, right)
                .done(before(left)),
              kv("last — char → index", last) ]);

        if map_has(last, c) && map_get(last, c) >= left {
            let j = map_get(last, c);
            step("check", `'${c}' already appears inside the window, at index ${j}.`,
                [ array("s", s).pointers(#{ left: left, right: right }).window(left, right)
                    .bad([j, right]).done(before(left)),
                  kv("last — char → index", last).hl([c]) ]);
            left = j + 1;
            set(#{ left: N(left) });
            log(`repeat '${c}' at ${j} -> left jumps to ${left}`);
            step("shrink",
                `Jump left straight to ${left}, just past that copy. Sliding one step at a time would revisit characters we already know are fine.`,
                [ array("s", s).pointers(#{ left: left, right: right }).window(left, right)
                    .done(before(left)),
                  kv("last — char → index", last).hl([c]) ]);
        } else {
            step("check", `'${c}' is not inside the current window.`,
                [ array("s", s).pointers(#{ left: left, right: right }).window(left, right)
                    .done(before(left)),
                  kv("last — char → index", last) ]);
        }

        map_set(last, c, right);
        set(#{ last: MAP(last, [c]) });
        step("store", `Record last['${c}'] = ${right}.`,
            [ array("s", s).pointers(#{ left: left, right: right }).window(left, right)
                .done(before(left)),
              kv("last — char → index", last).hl([c]) ]);

        let len = right - left + 1;
        if len > best {
            step("cmp", `The window is ${len} long, beating ${best}.`,
                [ array("s", s).pointers(#{ left: left, right: right }).hl(span(left, right))
                    .done(before(left)),
                  kv("last — char → index", last) ]);
            best = len;
            best_lo = left;
            set(#{ best: N(best) });
            log(`new best ${best} at [${best_lo}..${right}]`, "result");
            step("newBest", `Best is now ${best}.`,
                [ array("s", s).pointers(#{ left: left, right: right }).hl(span(left, right))
                    .done(before(left)),
                  kv("last — char → index", last) ]);
        } else {
            step("cmp", `The window is ${len} long, which does not beat ${best}.`,
                [ array("s", s).pointers(#{ left: left, right: right }).window(left, right)
                    .done(before(left)),
                  kv("last — char → index", last) ]);
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `The longest repeat-free stretch is ${best} characters, starting at index ${best_lo}.`,
        [ array("s", s).hl(span(best_lo, best_lo + best - 1)),
          kv("last — char → index", last) ],
        "return");
    leave(`${best}`);
}
