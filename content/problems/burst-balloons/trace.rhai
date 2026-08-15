// Burst Balloons — choose the balloon to burst *last*, not first.
//
// "Which do I burst first?" produces two halves that are still entangled: the
// left half's rightmost neighbour depends on what the right half does. "Which
// do I burst last?" does not. At the moment k is burst, everything else in
// [left..right] is already gone, so its neighbours are the untouched balloons
// just outside the range — fixed, known, independent of both subproblems. The
// virtual 1s at either end make the outermost ranges behave like the rest.

fn zeros(n) {
    let g = [];
    for r in 0..n {
        let line = [];
        for c in 0..n { line.push(0); }
        g.push(line);
    }
    g
}

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("maxCoins", #{ n: N(n) });

    let vals = [1];
    for v in nums { vals.push(v); }
    vals.push(1);
    step("pad", "A virtual balloon worth 1 at each end. They are never burst; they exist so that every real range has neighbours to multiply by.",
        [ bars("padded balloons", vals) ]);

    let dp = zeros(n + 2);
    step("init", "dp[left][right] is the most obtainable by bursting everything strictly inside that range of the padded array.",
        [ grid("dp[left][right]", dp) ]);

    let length = 1;
    while length <= n {
        step("len", `Ranges of ${plural(length, "balloon")}. Every longer range will read these, so they must be finished first.`,
            [ bars("padded balloons", vals), grid("dp[left][right]", dp) ]);

        let left = 1;
        while left + length - 1 <= n {
            step("left", `Range starting at ${left}.`,
                [ bars("padded balloons", vals).ptr("left", left),
                  grid("dp[left][right]", dp).cur([left, left + length - 1]) ]);

            let right = left + length - 1;
            step("right", `It ends at ${right}. Outside it sit ${vals[left - 1]} and ${vals[right + 1]}, and those never change while this range is being cleared.`,
                [ bars("padded balloons", vals).hl(span(left, right))
                    .pointers(#{ wall: left - 1, wall2: right + 1 }),
                  grid("dp[left][right]", dp).cur([left, right]) ]);

            let k = left;
            while k <= right {
                step("last", `Suppose balloon ${k} (worth ${vals[k]}) is the *last* one burst in this range.`,
                    [ bars("padded balloons", vals).hl([k]).window(left - 1, right + 1) ]);

                let gain = vals[left - 1] * vals[k] * vals[right + 1];
                step("gain", `By then everything else here is gone, so its neighbours are the walls: ${vals[left - 1]} × ${vals[k]} × ${vals[right + 1]} = ${gain}.`,
                    [ bars("padded balloons", vals).hl([left - 1, k, right + 1]),
                      text("this choice", [`last = ${k}`, `gain = ${gain}`]) ]);

                let total = dp[left][k - 1] + gain + dp[k + 1][right];
                step("total", `Left part earns ${dp[left][k - 1]}, right part ${dp[k + 1][right]}, and they cannot interfere because ${vals[k]} still separates them. Total ${total}.`,
                    [ grid("dp[left][right]", dp).cur([left, right])
                        .hl([[left, k - 1], [k + 1, right]]),
                      text("this choice", [`left: ${dp[left][k - 1]}`, `burst k: ${gain}`,
                                           `right: ${dp[k + 1][right]}`, `total: ${total}`]) ]);

                if total > dp[left][right] {
                    step("better", `${total} beats the ${dp[left][right]} found so far for this range.`,
                        [ grid("dp[left][right]", dp).cur([left, right]) ]);
                    grid_set(dp, left, right, total);
                    log(`dp[${left}][${right}] = ${total} (last = ${k})`);
                    step("store", `dp[${left}][${right}] = ${total}.`,
                        [ grid("dp[left][right]", dp).cur([left, right]).hl([[left, right]]) ]);
                } else {
                    step("better", `${total} does not beat ${dp[left][right]}.`,
                        [ grid("dp[left][right]", dp).cur([left, right]) ]);
                }

                k += 1;
            }
            left += 1;
        }
        length += 1;
    }

    let answer = dp[1][n];
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `Bursting every balloon in the best order earns ${answer}.`,
        [ bars("padded balloons", vals), grid("dp[left][right]", dp).hl([[1, n]]) ], "return");
    leave(`${answer}`);
}
