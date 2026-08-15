// Task Scheduler — the busiest task sets the shape of the schedule.
//
// Lay the most frequent task out first: it creates (maxCount - 1) gaps, each
// n+1 wide. Everything else fills those gaps. So the answer is either that
// skeleton's length, or — if there are more tasks than the skeleton has room
// for — simply the number of tasks, because no idling is ever needed.

fn trace(input) {
    let tasks = words(input.tasks);
    let n = input.n;
    let count = map_new();
    let max_count = 0;

    enter("leastInterval", #{ n: N(n) });
    step("count", `${tasks.len()} task(s), cooldown ${n}.`,
        [ array("tasks", tasks), kv("count", count) ]);
    set(#{ maxCount: N(max_count) });
    step("maxInit", "Only the most frequent task constrains the schedule.",
        [ kv("count", count) ]);

    for i in 0..tasks.len() {
        step("tally", `Task ${tasks[i]}.`,
            [ array("tasks", tasks).ptr("i", i).hl([i]).done(before(i)), kv("count", count) ]);
        let c = map_inc(count, tasks[i], 1);
        step("inc", `count[${tasks[i]}] = ${c}.`,
            [ array("tasks", tasks).ptr("i", i), kv("count", count).hl([tasks[i]]) ]);
        if c > max_count {
            step("cmpMax", `${tasks[i]} is now the most frequent.`, [ kv("count", count).hl([tasks[i]]) ]);
            max_count = c;
            set(#{ maxCount: N(max_count) });
            step("setMax", `maxCount = ${max_count}.`, [ kv("count", count) ]);
        }
    }

    let ties = 0;
    set(#{ ties: N(ties) });
    step("ties", `How many tasks occur ${max_count} times? Each needs a slot in the final block.`,
        [ kv("count", count) ]);

    for key in map_keys(count) {
        step("tieLoop", `${key} occurs ${map_get(count, key)} time(s).`,
            [ kv("count", count).hl([key]) ]);
        if map_get(count, key) == max_count {
            step("isTie", `That ties the maximum.`, [ kv("count", count).hl([key]) ]);
            ties += 1;
            set(#{ ties: N(ties) });
            step("addTie", `${ties} tied task(s).`, [ kv("count", count) ]);
        }
    }

    let slots = (max_count - 1) * (n + 1) + ties;
    set(#{ slots: N(slots) });
    step("formula",
        `The busiest task creates ${max_count - 1} gap(s) of width ${n + 1}, plus ${ties} final slot(s): (${max_count} − 1) × (${n} + 1) + ${ties} = ${slots}.`,
        [ text("schedule skeleton", [`gaps: ${max_count - 1} × ${n + 1}`, `tail: ${ties}`, `total: ${slots}`]) ]);

    if slots < tasks.len() {
        step("compare", `There are ${tasks.len()} tasks, more than the ${slots} slots — so the gaps fill up completely.`,
            [ array("tasks", tasks) ]);
        log(`answer ${tasks.len()}`, "result");
        result(`${tasks.len()}`);
        step("noIdle", `No idling is needed: ${tasks.len()}.`, [ array("tasks", tasks) ], "return");
        leave(`${tasks.len()}`);
        return;
    }

    step("compare", `Only ${tasks.len()} tasks for ${slots} slots, so some intervals must be idle.`,
        [ array("tasks", tasks) ]);
    log(`answer ${slots}`, "result");
    result(`${slots}`);
    step("ret", `${slots} interval(s).`,
        [ text("schedule skeleton", [`total: ${slots}`]) ], "return");
    leave(`${slots}`);
}
