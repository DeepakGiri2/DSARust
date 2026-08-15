// Product of Array Except Self — two sweeps, no division.
//
// The result array is used as scratch space: after the forward pass res[i]
// holds the product of everything to the *left* of i, and the backward pass
// multiplies in everything to the right. Watching the same cell get its second
// factor is the moment the trick lands.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let res = [];
    for i in 0..n { res.push(0); }

    enter("productExceptSelf", #{ nums: A(nums) });
    set(#{ n: N(n) });
    step("n", `${n} values. The answer never needs division — two passes are enough.`,
        [ array("nums", nums), array("res", res) ]);

    set(#{ res: A(res) });
    step("res", "res will first hold prefix products, then absorb the suffixes.",
        [ array("nums", nums), array("res", res) ]);

    let prefix = 1;
    set(#{ prefix: N(prefix) });
    step("pre", "prefix is the product of everything seen so far — nothing yet, so 1.",
        [ array("nums", nums), array("res", res) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("fwd", `Forward pass, i = ${i}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              array("res", res).done(before(i)) ]);

        res[i] = prefix;
        set(#{ res: A(res) });
        log(`res[${i}] = ${prefix} (product of everything left of ${i})`);
        step("setpre", `res[${i}] = ${prefix}: the product of everything strictly left of index ${i}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              array("res", res).hl([i]).done(before(i)) ]);

        prefix = prefix * nums[i];
        set(#{ prefix: N(prefix) });
        step("mulpre", `Fold nums[${i}] = ${nums[i]} in: prefix becomes ${prefix}.`,
            [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)),
              array("res", res).done(before(i + 1)) ]);
    }

    let postfix = 1;
    set(#{ postfix: N(postfix) });
    step("post", "Now the other direction. postfix is the product of everything to the right — again 1 to start.",
        [ array("nums", nums), array("res", res) ]);

    let i = n - 1;
    while i >= 0 {
        set(#{ i: N(i) });
        step("bwd", `Backward pass, i = ${i}.`,
            [ array("nums", nums).ptr("i", i).done(after(i, n)),
              array("res", res).done(after(i, n)) ]);

        let old = res[i];
        res[i] = res[i] * postfix;
        set(#{ res: A(res) });
        log(`res[${i}] = ${old} * ${postfix} = ${res[i]}`);
        step("setpost",
            `res[${i}] already held the left product ${old}; multiplying by the right product ${postfix} gives ${res[i]} — everything except nums[${i}].`,
            [ array("nums", nums).ptr("i", i).done(after(i, n)),
              array("res", res).hl([i]).done(after(i, n)) ]);

        postfix = postfix * nums[i];
        set(#{ postfix: N(postfix) });
        step("mulpost", `Fold nums[${i}] = ${nums[i]} into postfix: ${postfix}.`,
            [ array("nums", nums).ptr("i", i).hl([i]).done(after(i, n)),
              array("res", res).done(after(i - 1, n)) ]);
        i -= 1;
    }

    log(`answer ${brackets(res)}`, "result");
    result(brackets(res));
    step("return", `Every cell is the product of its neighbours on both sides: ${brackets(res)}.`,
        [ array("nums", nums), array("res", res).hl(seq(0, n - 1)) ], "return");
    leave(brackets(res));
}
