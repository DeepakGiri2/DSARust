// Stock With Cooldown — the cooldown is a state, not a counter.
//
// Trying to remember "how many days since the last sale" makes the state grow.
// Naming the three situations instead makes it constant: holding a share,
// having sold today, or free. A buy is only reachable from *free*, and the day
// after a sale you are in "just sold" — so the one-day wait is enforced by the
// shape of the transitions rather than by any arithmetic.

fn trace(input) {
    let prices = input.prices;
    let n = prices.len();

    enter("maxProfit", #{ n: N(n) });

    let hold = -prices[0];
    let sold = 0;
    let rest = 0;
    let holds = [hold];
    let solds = [sold];
    let rests = [rest];
    set(#{ hold: N(hold), sold: N(sold), rest: N(rest) });
    step("init", `On day 0 the only meaningful action is buying, which leaves you ${hold} down. Selling or resting are both worth 0.`,
        [ bars("prices", prices).ptr("day", 0),
          text("states after day 0", [`hold  = ${hold}`, `sold  = ${sold}`, `rest  = ${rest}`]) ]);

    for i in 1..n {
        step("loop", `Day ${i}, price ${prices[i]}. These are the values as of day ${i - 1}.`,
            [ bars("prices", prices).ptr("day", i).done(before(i)),
              text("states", [`hold  = ${hold}`, `sold  = ${sold}`, `rest  = ${rest}`]) ]);

        let ph = hold;
        let ps = sold;
        let pr = rest;
        step("snapshot", "Take yesterday's three values first — each of today's is written from them, so updating in place would let one leak into another.",
            [ text("yesterday", [`hold  = ${ph}`, `sold  = ${ps}`, `rest  = ${pr}`]) ]);

        hold = if ph > pr - prices[i] { ph } else { pr - prices[i] };
        set(#{ hold: N(hold) });
        step("hold", `Holding today means either still holding from yesterday (${ph}), or buying today from the *free* state (${pr} − ${prices[i]} = ${pr - prices[i]}). Buying from "sold" is not an option, and that is the cooldown. Best: ${hold}.`,
            [ bars("prices", prices).ptr("day", i).hl([i]),
              text("today", [`hold  = ${hold}`]) ]);

        sold = ph + prices[i];
        set(#{ sold: N(sold) });
        step("sold", `Selling today is only possible if you were holding yesterday: ${ph} + ${prices[i]} = ${sold}.`,
            [ text("today", [`hold  = ${hold}`, `sold  = ${sold}`]) ]);

        rest = if pr > ps { pr } else { ps };
        set(#{ rest: N(rest) });
        step("rest", `Being free today means you were free yesterday (${pr}) or sold yesterday and have now served the cooldown (${ps}). Best: ${rest}.`,
            [ text("today", [`hold  = ${hold}`, `sold  = ${sold}`, `rest  = ${rest}`]) ]);

        holds.push(hold);
        solds.push(sold);
        rests.push(rest);
    }

    let answer = if sold > rest { sold } else { rest };
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `Ending while still holding a share is never best, so the answer is the better of sold (${sold}) and rest (${rest}): ${answer}.`,
        [ bars("prices", prices),
          text("final states", [`hold = ${hold}`, `sold = ${sold}`, `rest = ${rest}`]) ], "return");
    leave(`${answer}`);
}
