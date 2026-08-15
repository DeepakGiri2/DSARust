// Concatenation of Array — index arithmetic, nothing else.
//
// Worth watching once because it makes the pattern explicit: position i and
// position i+n hold the same value, so one loop fills both halves.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("getConcatenation", #{ nums: A(nums) });
    set(#{ n: N(n) });
    step("n", `${n} value(s) in, ${2 * n} out.`, [ array("nums", nums) ]);

    let res = [];
    for i in 0..(2 * n) { res.push(0); }
    set(#{ ans: A(res) });
    step("alloc", `Allocate the answer up front — its size is known: 2 × ${n}.`,
        [ array("nums", nums), array("ans", res) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `i = ${i}: nums[${i}] = ${nums[i]} goes in two places.`,
            [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)),
              array("ans", res).done(before(i)) ]);

        res[i] = nums[i];
        set(#{ ans: A(res) });
        step("first", `ans[${i}] = ${nums[i]} — the first copy.`,
            [ array("nums", nums).ptr("i", i).hl([i]),
              array("ans", res).hl([i]) ]);

        res[i + n] = nums[i];
        set(#{ ans: A(res) });
        step("second", `ans[${i} + ${n}] = ans[${i + n}] = ${nums[i]} — the second.`,
            [ array("nums", nums).ptr("i", i).hl([i]),
              array("ans", res).hl([i, i + n]) ]);
    }

    log(`answer ${brackets(res)}`, "result");
    result(brackets(res));
    step("ret", `One pass, both halves filled: ${brackets(res)}.`,
        [ array("ans", res).hl(seq(0, 2 * n - 1)) ], "return");
    leave(brackets(res));
}
