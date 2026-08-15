// Coin Change II — the loop order is the algorithm.
//
// With coins on the outside, a combination is only ever built in one order:
// all the 1s, then all the 2s, then all the 5s. So 1+2 and 2+1 are the same
// path through the table and get counted once. Put amounts on the outside and
// the same code counts *permutations* instead — same arithmetic, different
// question. Scanning amounts upwards lets dp[a-c] already include this coin,
// which is what makes a coin reusable.

fn totals(t) {
    let out = [];
    for i in 0..(t + 1) { out.push(i); }
    out
}

fn trace(input) {
    let amount = input.amount;
    let coins = input.coins;

    enter("change", #{ amount: N(amount) });

    let dp = [];
    for a in 0..(amount + 1) { dp.push(0); }
    step("init", "dp[a] counts the combinations that make a, using only the coins introduced so far.",
        [ array("coins", coins), array("amount", totals(amount)), array("dp (combinations)", dp) ]);

    dp[0] = 1;
    step("seed", "There is exactly one way to make 0: take nothing. Every other count grows out of this one.",
        [ array("dp (combinations)", dp).hl([0]) ]);

    for k in 0..coins.len() {
        let c = coins[k];
        step("coinLoop", `Introduce the ${c}-coin. From now on every combination counted may include it — and because coins are introduced in a fixed order, no combination is ever counted twice.`,
            [ array("coins", coins).ptr("k", k).done(before(k)),
              array("dp (combinations)", dp) ]);

        let a = c;
        while a <= amount {
            step("amountLoop", `Amount ${a}.`,
                [ array("dp (combinations)", dp).ptr("a", a) ]);

            let ways = dp[a - c];
            step("lookup", `Every combination making ${a - c} becomes one making ${a} by adding this ${c}-coin — and dp[${a - c}] already counts combinations that use ${c}, which is how a coin gets reused.`,
                [ array("dp (combinations)", dp).pointers(#{ from: a - c, a: a }).hl([a - c]) ]);

            dp[a] = dp[a] + ways;
            log(`dp[${a}] = ${dp[a]}`);
            step("add", `dp[${a}] = ${dp[a]}.`,
                [ array("dp (combinations)", dp).ptr("a", a).hl([a]) ]);

            a += 1;
        }
    }

    log(`answer ${dp[amount]}`, "result");
    result(`${dp[amount]}`);
    step("ret", `${plural(dp[amount], "combination")} make ${amount}.`,
        [ bars("dp (combinations)", dp).ptr("amount", amount).hl([amount]) ], "return");
    leave(`${dp[amount]}`);
}
