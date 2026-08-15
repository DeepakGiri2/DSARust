// Gas Station — one pass, two independent facts.
//
// `total` answers whether *any* start works: the circuit is possible exactly
// when the gas gained overall covers the cost. `tank` finds *which* one: if the
// tank goes negative at index i, then no station from the current start up to
// i can work either — each of them would arrive at i with even less fuel — so
// the start jumps straight past the whole run.

fn diffs(gas, cost) {
    let out = [];
    for i in 0..gas.len() { out.push(gas[i] - cost[i]); }
    out
}

fn trace(input) {
    let gas = input.gas;
    let cost = input.cost;
    let n = gas.len();
    let d = diffs(gas, cost);

    let total = 0;
    let tank = 0;
    let start = 0;

    enter("canCompleteCircuit", #{});
    set(#{ total: N(total), tank: N(tank), start: N(start) });
    step("init", "total decides whether any start works; tank decides which one.",
        [ array("gas", gas), array("cost", cost), bars("gas − cost", d) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `Station ${i}.`,
            [ bars("gas − cost", d).pointers(#{ i: i, start: start }).done(before(i)) ]);

        step("diff", `It gives ${gas[i]} and costs ${cost[i]}, a net of ${d[i]}.`,
            [ array("gas", gas).ptr("i", i).hl([i]), array("cost", cost).ptr("i", i).hl([i]),
              bars("gas − cost", d).ptr("i", i).hl([i]) ]);

        total += d[i];
        set(#{ total: N(total) });
        step("total", `Overall balance so far: ${total}.`,
            [ bars("gas − cost", d).ptr("i", i).done(before(i + 1)) ]);

        tank += d[i];
        set(#{ tank: N(tank) });
        step("tank", `Tank starting from station ${start}: ${tank}.`,
            [ bars("gas − cost", d).pointers(#{ i: i, start: start }).window(start, i) ]);

        if tank < 0 {
            step("check", `The tank went negative at ${i}, so starting from ${start} fails.`,
                [ bars("gas − cost", d).pointers(#{ i: i, start: start })
                    .bad(span(start, i)) ]);
            start = i + 1;
            set(#{ start: N(start) });
            log(`start jumps to ${start}`);
            step("restart",
                `Every station between ${start - 1} and ${i} would reach ${i} with even less fuel, so they all fail too — skip straight to ${start}.`,
                [ bars("gas − cost", d).ptr("start", start).done(before(start)) ]);
            tank = 0;
            set(#{ tank: N(tank) });
            step("reset", "Empty tank at the new start.",
                [ bars("gas − cost", d).ptr("start", start).done(before(start)) ]);
        } else {
            step("check", `The tank is still at ${tank} — station ${start} is alive.`,
                [ bars("gas − cost", d).pointers(#{ i: i, start: start }).window(start, i) ]);
        }
    }

    if total < 0 {
        step("feasible", `The overall balance is ${total}: less gas than cost, so no start can work.`,
            [ bars("gas − cost", d).bad(seq(0, n - 1)) ]);
        log("no valid start", "result");
        result("-1");
        step("none", "Return -1.", [ bars("gas − cost", d) ], "return");
        leave("-1");
        return;
    }

    step("feasible", `The overall balance is ${total}, so a start exists — and only one candidate survived.`,
        [ bars("gas − cost", d).hl([start]) ]);
    log(`answer ${start}`, "result");
    result(`${start}`);
    step("ret", `Start at station ${start}.`,
        [ bars("gas − cost", d).hl([start]).done(before(start)) ], "return");
    leave(`${start}`);
}
