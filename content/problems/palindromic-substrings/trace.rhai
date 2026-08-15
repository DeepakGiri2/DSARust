// Palindromic Substrings — same centres, but count instead of measure.
//
// Every successful step outwards from a centre *is* a palindrome, so the count
// falls out of the same expansion that finds the longest one. No table, no
// re-checking.

fn trace(input) {
    let cs = chars(input.s);
    let n = cs.len();
    let count = 0;

    enter("countSubstrings", #{ s: S(input.s) });
    set(#{ count: N(count) });
    step("init", `Try all ${2 * n - 1} centres.`, [ array("s", cs) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `Centre ${i}.`, [ array("s", cs).ptr("i", i) ]);

        // Odd-length centres.
        enter(`expand(${i}, ${i})`);
        let found = 0;
        step("expInit", `Start from the single character '${cs[i]}'.`,
            [ array("s", cs).hl([i]) ]);
        let l = i;
        let r = i;
        while l >= 0 && r < n && cs[l] == cs[r] {
            step("grow", `s[${l}] and s[${r}] are both '${cs[l]}' — still symmetric.`,
                [ array("s", cs).pointers(#{ l: l, r: r }).hl(span(l, r)) ]);
            found += 1;
            count += 1;
            set(#{ count: N(count) });
            log(`"${join(cs.extract(l, r - l + 1), "")}" is a palindrome`);
            step("hit", `That is one more palindrome: "${join(cs.extract(l, r - l + 1), "")}". Total ${count}.`,
                [ array("s", cs).hl(span(l, r)) ]);
            l -= 1;
            r += 1;
        }
        step("expRet", `${found} odd-length palindrome(s) centred here.`,
            [ array("s", cs).ptr("i", i) ], "return");
        leave(`${found}`);
        step("odd", `Odd centres done for ${i}.`, [ array("s", cs).ptr("i", i) ]);

        // Even-length centres.
        if i + 1 < n {
            enter(`expand(${i}, ${i + 1})`);
            let found2 = 0;
            step("expInit", `Now the gap between ${i} and ${i + 1}.`,
                [ array("s", cs).pointers(#{ l: i, r: i + 1 }) ]);
            let l2 = i;
            let r2 = i + 1;
            while l2 >= 0 && r2 < n && cs[l2] == cs[r2] {
                step("grow", `s[${l2}] and s[${r2}] are both '${cs[l2]}'.`,
                    [ array("s", cs).pointers(#{ l: l2, r: r2 }).hl(span(l2, r2)) ]);
                found2 += 1;
                count += 1;
                set(#{ count: N(count) });
                log(`"${join(cs.extract(l2, r2 - l2 + 1), "")}" is a palindrome`);
                step("hit", `Total ${count}.`, [ array("s", cs).hl(span(l2, r2)) ]);
                l2 -= 1;
                r2 += 1;
            }
            step("expRet", `${found2} even-length palindrome(s) centred there.`,
                [ array("s", cs) ], "return");
            leave(`${found2}`);
            step("even", `Even centre done.`, [ array("s", cs).ptr("i", i) ]);
        }
    }

    log(`answer ${count}`, "result");
    result(`${count}`);
    step("ret", `${count} palindromic substring(s) in total.`,
        [ array("s", cs).done(seq(0, n - 1)) ], "return");
    leave(`${count}`);
}
