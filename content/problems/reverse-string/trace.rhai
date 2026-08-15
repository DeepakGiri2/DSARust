// Reverse String — swap from both ends inwards.
//
// n/2 swaps, no second array: each swap puts *two* characters in their final
// place, which is why the pointers only ever need to reach the middle.

fn trace(input) {
    let cs = chars(input.s);
    let n = cs.len();

    enter("reverseString", #{ s: S(input.s) });
    step("init", "Work on the characters in place — no copy is needed.", [ array("s", cs) ]);

    let l = 0;
    let r = n - 1;
    set(#{ l: N(l), r: N(r) });
    step("bounds", "A pointer at each end.", [ array("s", cs).pointers(#{ l: l, r: r }) ]);

    while l < r {
        step("loop", `${l} < ${r}, so there is still a pair to swap.`,
            [ array("s", cs).pointers(#{ l: l, r: r }).window(l, r).done(dead(n, l, r)) ]);

        let a = cs[l];
        let b = cs[r];
        swap_at(cs, l, r);
        log(`swap '${a}' and '${b}'`);
        step("swap", `Swap '${a}' and '${b}'. Both characters are now in their final position.`,
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

    let out = join(cs, "");
    log(`result ${out}`, "result");
    result(out);
    step("ret", `The pointers met, so every pair has been swapped: "${out}".`,
        [ array("s", cs).hl(seq(0, n - 1)) ], "return");
    leave(out);
}
