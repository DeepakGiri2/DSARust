// Counting Bits — every answer is one lookup away from a smaller one.
//
// i >> 1 is i with its lowest bit dropped, and that number has already been
// solved. So the count for i is the count for i >> 1 plus whatever the dropped
// bit was. No number is ever counted from scratch.

fn trace(input) {
    let n = input.n;
    let width = 5;
    let dp = [];
    for i in 0..(n + 1) { dp.push(0); }

    enter("countBits", #{ n: N(n) });
    set(#{ dp: A(dp) });
    step("init", "dp[0] = 0: zero has no bits set.",
        [ array("dp — number → 1 bits", dp).hl([0]) ]);

    for i in 1..(n + 1) {
        set(#{ i: N(i) });
        step("loop", `i = ${i}.`,
            [ array("dp — number → 1 bits", dp).ptr("i", i).done(before(i)),
              bits("halving", [[`i = ${i}`, i, width], [`i >> 1 = ${i >> 1}`, i >> 1, width]]) ]);

        let half = i >> 1;
        let low = i & 1;
        dp[i] = dp[half] + low;
        set(#{ dp: A(dp) });
        log(`dp[${i}] = dp[${half}] + ${low} = ${dp[i]}`);
        step("recur",
            `Dropping the lowest bit gives ${half}, already solved with ${dp[half]} bit(s). Add the dropped bit (${low}): dp[${i}] = ${dp[i]}.`,
            [ array("dp — number → 1 bits", dp).hl([i]).done([half]),
              bits("halving", [[`i = ${i}`, i, width, [0]], [`i >> 1 = ${half}`, half, width]]) ]);
    }

    let answer = join(dp, " ");
    result(answer);
    step("ret", `Every count in one pass: ${answer}.`,
        [ array("dp — number → 1 bits", dp).hl(seq(0, n)) ], "return");
    leave(answer);
}
