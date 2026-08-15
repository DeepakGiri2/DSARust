// Minimum Window Substring — grow to satisfy, shrink to minimise.
//
// The counter `have` is what keeps this linear: instead of re-comparing two
// maps on every move, it tracks how many *distinct* required letters are
// currently satisfied. The window is valid exactly when have == needCount.

fn trace(input) {
    let s = chars(input.s);
    let t = chars(input.t);
    let n = s.len();

    enter("minWindow", #{ s: S(input.s), t: S(input.t) });

    if t.len() == 0 {
        step("guard", "The needle is empty.", [ array("s", s) ]);
        result("\"\"");
        step("guardRet", "Return the empty string.", [ array("s", s) ], "return");
        leave("");
        return;
    }
    step("guard", `Looking for a window containing every letter of "${input.t}".`,
        [ array("s", s), array("t", t) ]);

    let need = map_new();
    step("need", "Tally what the needle requires.", [ array("t", t), kv("need", need) ]);
    for i in 0..t.len() {
        let c = map_inc(need, t[i], 1);
        step("buildNeed", `'${t[i]}' is needed.`,
            [ array("t", t).ptr("i", i).hl([i]), kv("need", need).hl([t[i]]) ]);
        step("needInc", `need['${t[i]}'] = ${c}.`, [ array("t", t).ptr("i", i), kv("need", need) ]);
    }

    let window = map_new();
    set(#{ window: MAP(window) });
    step("window", "The window's own tally.", [ array("s", s), kv("need", need), kv("window", window) ]);

    let have = 0;
    let need_count = map_len(need);
    set(#{ have: N(have), needCount: N(need_count) });
    step("have", `${need_count} distinct letter(s) must each reach their required count.`,
        [ array("s", s), kv("need", need), kv("window", window) ]);

    let res_l = -1;
    let res_r = -1;
    let res_len = n + 1;
    let left = 0;
    step("res", "No valid window found yet.", [ array("s", s) ]);
    set(#{ left: N(left) });
    step("left", "The window starts empty at index 0.", [ array("s", s).ptr("left", left) ]);

    for right in 0..n {
        let c = s[right];
        set(#{ right: N(right) });
        step("loop", `right = ${right}.`,
            [ array("s", s).pointers(#{ left: left, right: right })
                .window(left, right).done(before(left)),
              kv("window", window) ]);
        step("cur", `The character entering is '${c}'.`,
            [ array("s", s).pointers(#{ left: left, right: right }).hl([right]),
              kv("window", window) ]);

        let now = map_inc(window, c, 1);
        set(#{ window: MAP(window, [c]) });
        step("add", `window['${c}'] = ${now}.`,
            [ array("s", s).pointers(#{ left: left, right: right }).window(left, right),
              kv("need", need), kv("window", window).hl([c]) ]);

        if map_has(need, c) && now == map_get(need, c) {
            step("satisfy", `'${c}' has just reached the ${map_get(need, c)} copies the needle wants.`,
                [ array("s", s).pointers(#{ left: left, right: right }).window(left, right),
                  kv("need", need).hl([c]), kv("window", window).hl([c]) ]);
            have += 1;
            set(#{ have: N(have) });
            step("haveInc", `${have} of ${need_count} letters satisfied.`,
                [ array("s", s).pointers(#{ left: left, right: right }).window(left, right),
                  kv("window", window) ]);
        }

        while have == need_count {
            let len = right - left + 1;
            step("while", `Every required letter is present — the window [${left}..${right}] is valid.`,
                [ array("s", s).hl(span(left, right)), kv("window", window) ]);

            if len < res_len {
                step("better", `${len} is shorter than the best so far${if res_len > n { " (none yet)" } else { `` }}.`,
                    [ array("s", s).hl(span(left, right)) ]);
                res_l = left;
                res_r = right;
                step("save", `Remember [${res_l}..${res_r}].`, [ array("s", s).hl(span(res_l, res_r)) ]);
                res_len = len;
                log(`new best window "${join(s.extract(res_l, res_len), "")}"`, "result");
                step("saveLen", `Best length is now ${res_len}.`, [ array("s", s).hl(span(res_l, res_r)) ]);
            }

            let leaving = s[left];
            let after = map_inc(window, leaving, -1);
            set(#{ window: MAP(window, [leaving]) });
            step("shrink", `Try to do better: drop '${leaving}' from the left, window['${leaving}'] = ${after}.`,
                [ array("s", s).pointers(#{ left: left, right: right }).bad([left]),
                  kv("window", window).hl([leaving]) ]);

            if map_has(need, leaving) && after < map_get(need, leaving) {
                step("lose", `That was a letter the needle required — the window is no longer valid.`,
                    [ array("s", s).pointers(#{ left: left, right: right }).bad([left]),
                      kv("need", need).hl([leaving]) ]);
                have -= 1;
                set(#{ have: N(have) });
                step("haveDec", `${have} of ${need_count} satisfied.`,
                    [ array("s", s).pointers(#{ left: left, right: right }), kv("window", window) ]);
            }
            left += 1;
            set(#{ left: N(left) });
            step("advance", `left = ${left}.`,
                [ array("s", s).pointers(#{ left: left, right: right })
                    .window(left, right).done(before(left)),
                  kv("window", window) ]);
        }
    }

    if res_len > n {
        step("check", "No window ever contained every required letter.", [ array("s", s) ]);
        log("no valid window", "result");
        result("\"\"");
        step("empty", "Return the empty string.", [ array("s", s).done(seq(0, n - 1)) ], "return");
        leave("");
        return;
    }

    let answer = join(s.extract(res_l, res_len), "");
    step("check", `The shortest valid window is ${res_len} long.`, [ array("s", s).hl(span(res_l, res_r)) ]);
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `"${answer}" — [${res_l}..${res_r}].`, [ array("s", s).hl(span(res_l, res_r)) ], "return");
    leave(answer);
}
