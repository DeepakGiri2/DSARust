// Best Time to Buy & Sell Stock — one pass, two numbers.
//
// Drawn as bars because the shape of the price series is the intuition: the
// answer is the tallest rise from any earlier low to any later high, and the
// only thing worth remembering while scanning is the lowest price so far.

fn trace(input) {
    let prices = input.prices;
    let n = prices.len();

    let min_price = prices[0];
    let min_day = 0;
    let best = 0;
    let best_buy = 0;
    let best_sell = 0;

    enter("maxProfit", #{ prices: A(prices) });
    set(#{ minPrice: N(min_price), best: N(best) });
    step("init",
        `Day 0 is the only price seen so far, so the cheapest buy is ${min_price} and the best profit is 0 (do nothing).`,
        [ bars("prices", prices).ptr("min", 0).hl([0]) ]);

    for day in 1..n {
        let price = prices[day];
        set(#{ day: N(day) });
        step("loop", `Day ${day}: the price is ${price}.`,
            [ bars("prices", prices).pointers(#{ min: min_day, day: day }).done(before(day)) ]);

        let profit = price - min_price;
        set(#{ profit: N(profit) });
        log(`day ${day}: ${price} - ${min_price} = ${profit}`);
        step("profit",
            `Selling today after buying at the cheapest earlier price gives ${price} − ${min_price} = ${profit}.`,
            [ bars("prices", prices).pointers(#{ min: min_day, day: day })
                .hl([min_day, day]).done(before(day)) ]);

        if profit > best {
            step("cmpBest", `${profit} beats the best profit so far (${best}).`,
                [ bars("prices", prices).pointers(#{ min: min_day, day: day })
                    .hl([min_day, day]).done(before(day)) ]);
            best = profit;
            best_buy = min_day;
            best_sell = day;
            set(#{ best: N(best) });
            log(`new best profit ${best} (buy day ${best_buy}, sell day ${best_sell})`, "result");
            step("newBest", `Record it: buy on day ${best_buy}, sell on day ${best_sell}, profit ${best}.`,
                [ bars("prices", prices).pointers(#{ min: min_day, day: day })
                    .hl([best_buy, best_sell]).done(before(day)) ]);
        } else {
            step("cmpBest", `${profit} does not beat the best so far (${best}), so nothing to record.`,
                [ bars("prices", prices).pointers(#{ min: min_day, day: day })
                    .hl([best_buy, best_sell]).done(before(day)) ]);
        }

        if price < min_price {
            step("cmpMin", `${price} is cheaper than ${min_price} — a better day to have bought.`,
                [ bars("prices", prices).pointers(#{ min: min_day, day: day })
                    .bad([min_day]).done(before(day)) ]);
            min_price = price;
            min_day = day;
            set(#{ minPrice: N(min_price) });
            log(`new minimum ${min_price} on day ${min_day}`);
            step("newMin",
                `From here on, any sale is measured against ${min_price}. Note we never look backwards — one number carries the whole past.`,
                [ bars("prices", prices).pointers(#{ min: min_day, day: day }).done(before(day)) ]);
        } else {
            step("cmpMin", `${price} is not cheaper than ${min_price}, so the buy day stays put.`,
                [ bars("prices", prices).pointers(#{ min: min_day, day: day }).done(before(day)) ]);
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret",
        if best > 0 {
            `Best possible profit is ${best}: buy on day ${best_buy} at ${prices[best_buy]}, sell on day ${best_sell} at ${prices[best_sell]}.`
        } else {
            "Prices never rose after a low point, so the best move is not to trade at all: 0."
        },
        [ bars("prices", prices).hl([best_buy, best_sell]).done(before(n)) ],
        "return");
    leave(`${best}`);
}
