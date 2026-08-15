// Longest Palindromic Substring — expand around every centre.
//
// Every palindrome is symmetric about a middle, and there are only 2n-1
// possible middles: each character, and each gap between two. Growing outwards
// from a centre stops the moment the two sides disagree, so no substring is
// ever checked twice.

fn expand(cs, l, r) {
    while l >= 0 && r < cs.len() && cs[l] == cs[r] {
        l -= 1;
        r += 1;
    }
    [l + 1, r - 1]
}

fn trace(input) {
    let cs = chars(input.s);
    let n = cs.len();
    let best_lo = 0;
    let best_hi = -1;

    enter("longestPalindrome", #{ s: S(input.s) });
    set(#{ best: S("") });
    step("init", `${2 * n - 1} possible centres: ${n} characters and ${n - 1} gaps.`,
        [ array("s", cs) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `Centre ${i}.`,
            [ array("s", cs).ptr("i", i).hl(if best_hi >= best_lo { span(best_lo, best_hi) } else { [] }) ]);

        let odd = expand(cs, i, i);
        let odd_len = odd[1] - odd[0] + 1;
        step("odd", `Growing outwards from '${cs[i]}' gives a palindrome of length ${odd_len}.`,
            [ array("s", cs).ptr("i", i).hl(span(odd[0], odd[1])) ]);

        if odd_len > best_hi - best_lo + 1 {
            step("cmpOdd", `${odd_len} beats the best so far.`,
                [ array("s", cs).hl(span(odd[0], odd[1])) ]);
            best_lo = odd[0];
            best_hi = odd[1];
            set(#{ best: S(join(cs.extract(best_lo, best_hi - best_lo + 1), "")) });
            log(`new best "${join(cs.extract(best_lo, best_hi - best_lo + 1), "")}"`, "result");
            step("setOdd", `Best is now "${join(cs.extract(best_lo, best_hi - best_lo + 1), "")}".`,
                [ array("s", cs).hl(span(best_lo, best_hi)) ]);
        } else {
            step("cmpOdd", `${odd_len} does not beat ${best_hi - best_lo + 1}.`,
                [ array("s", cs).hl(span(best_lo, best_hi)) ]);
        }

        if i + 1 < n {
            let even = expand(cs, i, i + 1);
            let even_len = even[1] - even[0] + 1;
            step("even", `Now the gap between ${i} and ${i + 1} — an even-length palindrome needs its own centre.`,
                [ array("s", cs).pointers(#{ l: i, r: i + 1 })
                    .hl(if even_len > 0 { span(even[0], even[1]) } else { [] }) ]);

            if even_len > best_hi - best_lo + 1 {
                step("cmpEven", `${even_len} beats the best so far.`,
                    [ array("s", cs).hl(span(even[0], even[1])) ]);
                best_lo = even[0];
                best_hi = even[1];
                set(#{ best: S(join(cs.extract(best_lo, best_hi - best_lo + 1), "")) });
                log(`new best "${join(cs.extract(best_lo, best_hi - best_lo + 1), "")}"`, "result");
                step("setEven", `Best is now "${join(cs.extract(best_lo, best_hi - best_lo + 1), "")}".`,
                    [ array("s", cs).hl(span(best_lo, best_hi)) ]);
            } else {
                step("cmpEven", `${even_len} does not beat ${best_hi - best_lo + 1}.`,
                    [ array("s", cs).hl(span(best_lo, best_hi)) ]);
            }
        }
    }

    let answer = join(cs.extract(best_lo, best_hi - best_lo + 1), "");
    result(answer);
    step("ret", `The longest palindrome is "${answer}".`,
        [ array("s", cs).hl(span(best_lo, best_hi)) ], "return");
    leave(answer);
}
