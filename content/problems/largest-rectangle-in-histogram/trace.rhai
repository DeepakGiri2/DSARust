// Largest Rectangle in Histogram — a monotonic stack of (start, height).
//
// A bar's rectangle can keep growing right until something shorter appears.
// So when a shorter bar arrives, every taller bar on the stack is finished and
// its area can be settled on the spot. The subtle part is `inherit`: the new
// bar takes over the earliest start index it popped, because it can stretch
// back across all of them.

fn labels(stack) {
    let out = [];
    for e in stack { out.push(`h${e[1]}@${e[0]}`); }
    out
}

fn trace(input) {
    let h = input.heights;
    let n = h.len();
    let best = 0;
    let best_span = [];
    let stack = [];

    enter("largestRectangleArea", #{ heights: A(h) });
    set(#{ best: N(best) });
    step("init", "No rectangle measured yet.", [ bars("heights", h) ]);
    step("stack", "The stack holds bars that can still grow to the right, kept increasing in height.",
        [ bars("heights", h), stack("stack (height@start)", labels(stack)) ]);

    for i in 0..n {
        step("loop", `Bar ${i} is ${h[i]} tall.`,
            [ bars("heights", h).ptr("i", i).done(before(i)),
              stack("stack (height@start)", labels(stack)) ]);

        let start = i;
        step("start", `Assume for now that its rectangle starts at ${i}.`,
            [ bars("heights", h).ptr("i", i).hl([i]),
              stack("stack (height@start)", labels(stack)) ]);

        while stack.len() > 0 && stack[stack.len() - 1][1] > h[i] {
            let top = stack[stack.len() - 1];
            step("while", `The bar on top is ${top[1]} tall, higher than ${h[i]} — it can grow no further right.`,
                [ bars("heights", h).ptr("i", i).hl(span(top[0], i - 1)),
                  stack("stack (height@start)", labels(stack)) ]);
            step("peek", `It started at index ${top[0]}.`,
                [ bars("heights", h).ptr("i", i).hl(span(top[0], i - 1)),
                  stack("stack (height@start)", labels(stack)) ]);

            stack.truncate(stack.len() - 1);
            step("pop", "Take it off.",
                [ bars("heights", h).ptr("i", i),
                  stack("stack (height@start)", labels(stack)).popped(`h${top[1]}@${top[0]}`) ]);

            let area = top[1] * (i - top[0]);
            if area > best {
                best = area;
                best_span = span(top[0], i - 1);
                log(`new best area ${best}`, "result");
            }
            set(#{ best: N(best) });
            step("area", `Its rectangle is ${top[1]} × ${i - top[0]} = ${area}. Best so far: ${best}.`,
                [ bars("heights", h).hl(span(top[0], i - 1)).ptr("i", i),
                  stack("stack (height@start)", labels(stack)) ]);

            start = top[0];
            step("inherit",
                `Bar ${i} is shorter, so it can stretch back over everything that bar covered — its own start moves to ${start}.`,
                [ bars("heights", h).ptr("i", i).hl(span(start, i)),
                  stack("stack (height@start)", labels(stack)) ]);
        }

        stack.push([start, h[i]]);
        step("push", `Push bar ${i} with start ${start}.`,
            [ bars("heights", h).ptr("i", i).done(before(i + 1)),
              stack("stack (height@start)", labels(stack)).pushed(true) ]);
    }

    for e in stack {
        step("drain", `Bar of height ${e[1]} never met anything shorter, so it runs to the end.`,
            [ bars("heights", h).hl(span(e[0], n - 1)),
              stack("stack (height@start)", labels(stack)) ]);
        let area = e[1] * (n - e[0]);
        if area > best {
            best = area;
            best_span = span(e[0], n - 1);
            log(`new best area ${best}`, "result");
        }
        set(#{ best: N(best) });
        step("tail", `Area ${e[1]} × ${n - e[0]} = ${area}. Best: ${best}.`,
            [ bars("heights", h).hl(span(e[0], n - 1)) ]);
    }

    result(`${best}`);
    step("ret", `The largest rectangle has area ${best}. Every bar was pushed and popped once, so this is linear.`,
        [ bars("heights", h).hl(best_span) ], "return");
    leave(`${best}`);
}
