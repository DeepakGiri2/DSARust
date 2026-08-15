// Valid Palindrome II — one deletion allowed.
//
// Walk inwards until a mismatch. At that point exactly one of the two
// characters must be the one deleted, so there are only two candidate strings
// to check — and each check is a plain palindrome scan. No need to try
// deleting anywhere else.

fn is_range(cs, i, j) {
    while i < j {
        if cs[i] != cs[j] { return false; }
        i += 1;
        j -= 1;
    }
    true
}

fn trace(input) {
    let cs = chars(input.s);
    let n = cs.len();
    let l = 0;
    let r = n - 1;

    enter("validPalindrome", #{ s: S(input.s) });
    set(#{ l: N(l), r: N(r) });
    step("init", "Start as a normal palindrome check, with one deletion in reserve.",
        [ array("s", cs).pointers(#{ l: l, r: r }) ]);

    while l < r {
        step("loop", `Comparing positions ${l} and ${r}.`,
            [ array("s", cs).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);

        if cs[l] != cs[r] {
            step("cmp", `'${cs[l]}' and '${cs[r]}' differ. One of these two must be the character we delete — there is no third option.`,
                [ array("s", cs).pointers(#{ l: l, r: r }).bad([l, r]).done(dead(n, l, r)) ]);

            let skip_left = is_range(cs, l + 1, r);
            let skip_right = is_range(cs, l, r - 1);
            enter(`isRange`, #{ l: N(l), r: N(r) });
            step("rloop", `Try deleting '${cs[l]}': does [${l + 1}..${r}] read the same both ways? ${skip_left}.`,
                [ array("s", cs).pointers(#{ l: l + 1, r: r }).window(l + 1, r).bad([l]) ]);
            step("rcmp", `Try deleting '${cs[r]}': does [${l}..${r - 1}] read the same both ways? ${skip_right}.`,
                [ array("s", cs).pointers(#{ l: l, r: r - 1 }).window(l, r - 1).bad([r]) ]);

            let ok = skip_left || skip_right;
            if ok {
                log(`one deletion is enough`, "result");
                step("rgood", "One of the two halves is a palindrome, so a single deletion is enough.",
                    [ array("s", cs).hl(if skip_left { span(l + 1, r) } else { span(l, r - 1) }) ],
                    "return");
            } else {
                log("neither deletion helps", "result");
                step("rbad", "Neither deletion produces a palindrome, so one is not enough.",
                    [ array("s", cs).bad([l, r]) ], "return");
            }
            leave(`${ok}`);

            result(`${ok}`);
            step("branch", `Return ${ok}.`,
                [ array("s", cs).pointers(#{ l: l, r: r }) ], "return");
            leave(`${ok}`);
            return;
        }

        step("cmp", `'${cs[l]}' and '${cs[r]}' match — no deletion needed yet.`,
            [ array("s", cs).pointers(#{ l: l, r: r }).hl([l, r]).done(dead(n, l, r)) ]);
        l += 1;
        set(#{ l: N(l) });
        step("moveL", `l = ${l}.`,
            [ array("s", cs).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);
        r -= 1;
        set(#{ r: N(r) });
        step("moveR", `r = ${r}.`,
            [ array("s", cs).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);
    }

    log("already a palindrome", "result");
    result("true");
    step("good", "The pointers met with no mismatch at all — the deletion was never needed.",
        [ array("s", cs).hl(seq(0, n - 1)) ], "return");
    leave("true");
}
