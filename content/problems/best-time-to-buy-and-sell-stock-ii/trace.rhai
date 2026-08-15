// Best Time to Buy and Sell Stock II — take every rise, skip every fall.
//
// It looks like it should need to find the right peaks and valleys, and it
// does not. Holding from a valley to a peak earns exactly the sum of the daily
// rises in between, and any fall inside that hold can be avoided by selling
// the day before and buying back the day after — at no cost. So the maximum is
// simply the total of the positive day-to-day differences.

const P = "prices";

fn trace(input) {
    let prices = input.prices;
    let n = prices.len();

    enter("maxProfit", #{ prices: A(prices) });

    let profit = 0;
    let held = [];
    set(#{ profit: N(0) });
    step("init", "Profit starts at 0. There is no state to track beyond it — no buy price, no holding flag.",
        [ bars(global::P, prices), text("profit", ["0"]) ]);

    for i in 1..n {
        set(#{ i: N(i) });
        step("loop", `Day ${i}: ${prices[i - 1]} → ${prices[i]}.`,
            [ bars(global::P, prices).ptr("i", i).hl([i - 1, i]).done(held) ]);

        if prices[i] > prices[i - 1] {
            let gain = prices[i] - prices[i - 1];
            step("up", `The price rose by ${gain} — buy yesterday, sell today.`,
                [ bars(global::P, prices).ptr("i", i).hl([i - 1, i]).done(held) ]);
            profit += gain;
            held.push(i - 1);
            held.push(i);
            set(#{ profit: N(profit) });
            step("add", `profit = ${profit}. Selling and rebuying on the same day is free, so consecutive rises chain into one hold at no cost.`,
                [ bars(global::P, prices).done(held), text("profit", [`${profit}`]) ]);
        } else {
            step("up", `The price ${if prices[i] == prices[i - 1] { "did not move" } else { `fell by ${prices[i - 1] - prices[i]}` }} — skip it. Holding through a fall can only lose money.`,
                [ bars(global::P, prices).ptr("i", i).bad([i]).done(held) ]);
        }
    }

    log(`profit ${profit}`, "result");
    result(`${profit}`);
    step("ret", `Every rise collected: ${profit}.`,
        [ bars(global::P, prices).done(held), text("profit", [`${profit}`]) ], "return");
    leave(`${profit}`);
}
