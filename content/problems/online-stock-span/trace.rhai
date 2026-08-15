// Online Stock Span — a beaten price is dead, but its days live on.
//
// Once today's price matches or exceeds an earlier one, that earlier price can
// never limit any future day either — every future day that reaches today also
// reaches it. So it is discarded. The trick is that its *span* is added to
// today's first, which hands over a whole block of days in one step instead of
// re-walking them. That is what makes an apparently quadratic scan linear.

fn labels(stack) {
    let out = [];
    for e in stack { out.push(`${e[0]}·${e[1]}d`); }
    out
}

fn trace(input) {
    let prices = input.prices;
    let n = prices.len();
    let stack = [];
    let spans = [];

    enter("StockSpanner", #{});
    step("init", "A decreasing stack of (price, span) pairs — every entry is a price nothing since has beaten.",
        [ array("prices", prices), stack("stack (price·span)", labels(stack)) ]);

    for i in 0..n {
        let p = prices[i];
        enter(`next(${p})`, #{ price: N(p) });

        let span = 1;
        set(#{ span: N(span) });
        step("span", `Day ${i} costs ${p}. Today alone always counts, so the span starts at 1.`,
            [ array("prices", prices).ptr("i", i).hl([i]).done(before(i)),
              stack("stack (price·span)", labels(stack)) ]);

        while stack.len() > 0 && stack[stack.len() - 1][0] <= p {
            let top = stack[stack.len() - 1];
            step("while", `The stack's top price is ${top[0]}, which is not above ${p} — so it is beaten, and it can never limit a later day either.`,
                [ array("prices", prices).ptr("i", i).hl([i]),
                  stack("stack (price·span)", labels(stack)) ]);

            span += top[1];
            set(#{ span: N(span) });
            log(`absorb ${top[1]} day(s) from price ${top[0]}`);
            step("absorb", `Absorb its ${plural(top[1], "day")} in one move — that block is already known to be under ${p}.`,
                [ array("prices", prices).ptr("i", i).hl(span_back(i, span)),
                  stack("stack (price·span)", labels(stack)) ]);

            stack.truncate(stack.len() - 1);
            step("pop", "Discard it; it is of no further use to anyone.",
                [ array("prices", prices).ptr("i", i).hl(span_back(i, span)),
                  stack("stack (price·span)", labels(stack)).popped(`${top[0]}·${top[1]}d`) ]);
        }

        stack.push([p, span]);
        spans.push(span);
        step("push", `Push (${p}, ${span}). The stack stays strictly decreasing in price.`,
            [ array("prices", prices).ptr("i", i).hl(span_back(i, span)),
              stack("stack (price·span)", labels(stack)).pushed(true) ]);

        log(`span of day ${i} is ${span}`, "result");
        step("ret", `Span ${span}.`,
            [ array("prices", prices).ptr("i", i).hl(span_back(i, span)),
              array("spans", spans) ], "return");
        leave(`${span}`);
    }

    let answer = join(spans, " ");
    result(answer);
    leave(answer);
}

fn span_back(i, len) {
    span(i - len + 1, i)
}
