// Coin Change — bottom-up DP over every amount.
//
// dp[a] is the fewest coins that make exactly `a`. Each amount is solved by
// asking, for every coin, "what if this were the last coin I placed?" — which
// is a question about a smaller amount that has already been answered.

// Renders the "impossible" marker as `inf` instead of a magic number.
fn cells(dp, inf) {
    let out = [];
    for v in dp { out.push(if v >= inf { "inf" } else { `${v}` }); }
    out
}

fn trace(input) {
    let coins = input.coins;
    let amount = input.amount;
    let inf = amount + 1;

    let dp = [];
    for a in 0..(amount + 1) { dp.push(inf); }
    dp[0] = 0;

    enter("coinChange", #{ coins: A(coins), amount: N(amount) });
    set(#{ dp: A(cells(dp, inf)) });
    step("init", "dp[a] will hold the fewest coins that add up to exactly a. dp[0] = 0: zero coins make zero.",
        [ array("coins", coins), array("dp — amount → coins", cells(dp, inf)).hl([0]) ]);

    for a in 1..(amount + 1) {
        step("fill", `dp[${a}] starts unreachable.`,
            [ array("coins", coins),
              array("dp — amount → coins", cells(dp, inf)).ptr("a", a).done(before(a)) ]);
    }
    step("inf", `Everything above 0 starts at "impossible" (${inf}, one more than any real answer could be).`,
        [ array("coins", coins), array("dp — amount → coins", cells(dp, inf)) ]);

    for a in 1..(amount + 1) {
        set(#{ a: N(a) });
        step("outer", `Now solve amount ${a}.`,
            [ array("coins", coins),
              array("dp — amount → coins", cells(dp, inf)).ptr("a", a).done(before(a)) ]);

        for ci in 0..coins.len() {
            let c = coins[ci];
            step("inner", `What if the last coin placed were the ${c}?`,
                [ array("coins", coins).ptr("c", ci).hl([ci]),
                  array("dp — amount → coins", cells(dp, inf)).ptr("a", a).done(before(a)) ]);

            if c <= a && dp[a - c] + 1 < dp[a] {
                step("check",
                    `Using it leaves ${a} − ${c} = ${a - c}, which takes ${dp[a - c]} coin(s) — so ${a} is reachable in ${dp[a - c] + 1}, better than ${if dp[a] >= inf { "impossible" } else { `${dp[a]}` }}.`,
                    [ array("coins", coins).ptr("c", ci).hl([ci]),
                      array("dp — amount → coins", cells(dp, inf))
                        .ptr("a", a).hl([a - c]).done(before(a)) ]);
                dp[a] = dp[a - c] + 1;
                set(#{ dp: A(cells(dp, inf)) });
                log(`dp[${a}] = dp[${a - c}] + 1 = ${dp[a]}`);
                step("update", `dp[${a}] = ${dp[a]}.`,
                    [ array("coins", coins).ptr("c", ci).hl([ci]),
                      array("dp — amount → coins", cells(dp, inf)).hl([a]).done(before(a)) ]);
            } else if c > a {
                step("check", `The ${c} coin is larger than ${a}, so it cannot be the last coin.`,
                    [ array("coins", coins).ptr("c", ci).bad([ci]),
                      array("dp — amount → coins", cells(dp, inf)).ptr("a", a).done(before(a)) ]);
            } else {
                step("check", `That path needs ${dp[a - c] + 1} coin(s), no better than the ${dp[a]} already recorded.`,
                    [ array("coins", coins).ptr("c", ci),
                      array("dp — amount → coins", cells(dp, inf))
                        .ptr("a", a).hl([a - c]).done(before(a)) ]);
            }
        }
    }

    if dp[amount] > amount {
        step("test", `dp[${amount}] is still at the impossible marker.`,
            [ array("coins", coins),
              array("dp — amount → coins", cells(dp, inf)).bad([amount]) ]);
        log("no combination reaches the amount", "result");
        result("-1");
        step("none", `No combination of these coins adds up to ${amount}. Return -1.`,
            [ array("dp — amount → coins", cells(dp, inf)).bad([amount]) ], "return");
        leave("-1");
        return;
    }

    step("test", `dp[${amount}] = ${dp[amount]} is a real answer.`,
        [ array("coins", coins), array("dp — amount → coins", cells(dp, inf)).hl([amount]) ]);
    log(`answer ${dp[amount]}`, "result");
    result(`${dp[amount]}`);
    step("ret", `${amount} needs at least ${dp[amount]} coin(s).`,
        [ array("coins", coins), array("dp — amount → coins", cells(dp, inf)).hl([amount]) ],
        "return");
    leave(`${dp[amount]}`);
}
