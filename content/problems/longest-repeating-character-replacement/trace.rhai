// Longest Repeating Character Replacement — window length minus the most
// common letter in it equals the number of replacements it would take.
//
// That single expression is the whole problem: if it exceeds k the window is
// unaffordable and must shrink, otherwise it is a valid answer.

fn trace(input) {
    let cs = chars(input.s);
    let k = input.k;
    let n = cs.len();
    let label = `s   (k = ${k} replacements)`;
    let count = map_new();

    let left = 0;
    let maxf = 0;
    let best = 0;
    let best_lo = 0;

    enter("characterReplacement", #{ k: N(k) });
    set(#{ count: MAP(count) });
    step("init", "Letter counts inside the current window.",
        [ array(label, cs), kv("count", count) ]);
    set(#{ left: N(left), best: N(best), maxf: N(maxf) });
    step("init2", "maxf tracks the most common letter in the window — the one we would keep.",
        [ array(label, cs).ptr("left", left), kv("count", count) ]);

    for right in 0..n {
        let c = cs[right];
        set(#{ right: N(right) });
        step("loop", `right = ${right}: '${c}' joins the window.`,
            [ array(label, cs).pointers(#{ left: left, right: right })
                .window(left, right).done(before(left)),
              kv("count", count) ]);

        let seen = map_inc(count, c, 1);
        set(#{ count: MAP(count, [c]) });
        step("add", `count['${c}'] = ${seen}.`,
            [ array(label, cs).pointers(#{ left: left, right: right })
                .window(left, right).hl([right]).done(before(left)),
              kv("count", count).hl([c]) ]);

        if seen > maxf {
            step("max", `'${c}' is now the most common letter in the window.`,
                [ array(label, cs).pointers(#{ left: left, right: right }).window(left, right),
                  kv("count", count).hl([c]) ]);
            maxf = seen;
            set(#{ maxf: N(maxf) });
            step("setmax", `maxf = ${maxf}.`,
                [ array(label, cs).pointers(#{ left: left, right: right }).window(left, right),
                  kv("count", count) ]);
        }

        while (right - left + 1) - maxf > k {
            let need = (right - left + 1) - maxf;
            step("while",
                `The window is ${right - left + 1} long with ${maxf} of the same letter, so it needs ${need} replacements — more than the ${k} allowed.`,
                [ array(label, cs).pointers(#{ left: left, right: right })
                    .bad(span(left, right)).done(before(left)),
                  kv("count", count) ]);
            let after = map_inc(count, cs[left], -1);
            set(#{ count: MAP(count, [cs[left]]) });
            step("dec", `Drop '${cs[left]}' from the left: count['${cs[left]}'] = ${after}.`,
                [ array(label, cs).pointers(#{ left: left, right: right }).bad([left]),
                  kv("count", count).hl([cs[left]]) ]);
            left += 1;
            set(#{ left: N(left) });
            step("advance", `left = ${left}.`,
                [ array(label, cs).pointers(#{ left: left, right: right })
                    .window(left, right).done(before(left)),
                  kv("count", count) ]);
        }

        let len = right - left + 1;
        if len > best {
            step("cmp", `This window is ${len} long and affordable, beating ${best}.`,
                [ array(label, cs).hl(span(left, right)), kv("count", count) ]);
            best = len;
            best_lo = left;
            set(#{ best: N(best) });
            log(`new best ${best} at [${best_lo}..${right}]`, "result");
            step("newBest", `Best is now ${best}.`,
                [ array(label, cs).hl(span(best_lo, right)), kv("count", count) ]);
        } else {
            step("cmp", `${len} does not beat ${best}.`,
                [ array(label, cs).window(left, right).hl(span(best_lo, best_lo + best - 1)),
                  kv("count", count) ]);
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `The longest run obtainable with ${k} replacement(s) is ${best}.`,
        [ array(label, cs).hl(span(best_lo, best_lo + best - 1)) ], "return");
    leave(`${best}`);
}
