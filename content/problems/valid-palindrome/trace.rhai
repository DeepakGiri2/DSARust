// Valid Palindrome — two pointers that skip the noise.
//
// Characters that are not letters or digits never take part in the comparison,
// so the pointers walk *past* them instead of the string being cleaned up
// first. That is the whole saving: no second string, no extra memory.

fn is_alnum(c) {
    (c >= "0" && c <= "9") || (c >= "a" && c <= "z") || (c >= "A" && c <= "Z")
}

fn lower_of(c) {
    if c >= "A" && c <= "Z" { chr(ord(c) + 32) } else { c }
}

fn trace(input) {
    let cs = chars(input.s);
    let n = cs.len();
    let l = 0;
    let r = n - 1;

    enter("isPalindrome", #{ s: S(input.s) });
    set(#{ l: N(l), r: N(r) });
    step("init", "One pointer at each end. They walk towards each other, comparing as they go.",
        [ array("s", cs).pointers(#{ l: l, r: r }) ]);

    while l < r {
        step("loop", `The pointers have not met yet: ${l} < ${r}.`,
            [ array("s", cs).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);

        while l < r && !is_alnum(cs[l]) {
            step("skipL", `'${cs[l]}' is not a letter or digit — it plays no part in a palindrome.`,
                [ array("s", cs).pointers(#{ l: l, r: r }).bad([l]).done(dead(n, l, r)) ]);
            l += 1;
            set(#{ l: N(l) });
            step("incL", `Skip it: l = ${l}.`,
                [ array("s", cs).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);
        }
        while l < r && !is_alnum(cs[r]) {
            step("skipR", `'${cs[r]}' on the right is not a letter or digit either.`,
                [ array("s", cs).pointers(#{ l: l, r: r }).bad([r]).done(dead(n, l, r)) ]);
            r -= 1;
            set(#{ r: N(r) });
            step("decR", `Skip it: r = ${r}.`,
                [ array("s", cs).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);
        }
        if l >= r { break; }

        let a = lower_of(cs[l]);
        let b = lower_of(cs[r]);
        if a != b {
            log(`'${a}' != '${b}'`, "result");
            result("false");
            step("cmp", `'${cs[l]}' and '${cs[r]}' are different letters, so the string cannot read the same both ways.`,
                [ array("s", cs).pointers(#{ l: l, r: r }).bad([l, r]) ]);
            step("bad", "Return false.", [ array("s", cs).bad([l, r]) ], "return");
            leave("false");
            return;
        }

        step("cmp", `'${cs[l]}' and '${cs[r]}' match, ignoring case.`,
            [ array("s", cs).pointers(#{ l: l, r: r }).hl([l, r]).done(dead(n, l, r)) ]);
        l += 1;
        set(#{ l: N(l) });
        step("moveL", `l moves in to ${l}.`,
            [ array("s", cs).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);
        r -= 1;
        set(#{ r: N(r) });
        step("moveR", `r moves in to ${r}. The unchecked middle keeps shrinking.`,
            [ array("s", cs).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);
    }

    log("pointers met with every pair matching", "result");
    result("true");
    step("good", "The pointers met in the middle without a single mismatch — it is a palindrome.",
        [ array("s", cs).hl(seq(0, n - 1)) ], "return");
    leave("true");
}
