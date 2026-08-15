fn hpush_min(heap, key, label) {
    heap.push([key, label]);
    let i = heap.len() - 1;
    while i > 0 {
        let p = (i - 1) / 2;
        if heap[p][0] <= heap[i][0] { break; }
        swap_at(heap, p, i);
        i = p;
    }
    heap
}

fn hpop_min(heap) {
    let n = heap.len();
    heap[0] = heap[n - 1];
    heap.truncate(n - 1);
    let i = 0;
    loop {
        let l = 2 * i + 1;
        let r = 2 * i + 2;
        let small = i;
        if l < heap.len() && heap[l][0] < heap[small][0] { small = l; }
        if r < heap.len() && heap[r][0] < heap[small][0] { small = r; }
        if small == i { break; }
        swap_at(heap, i, small);
        i = small;
    }
    heap
}

fn hpush_max(heap, key, label) {
    heap.push([key, label]);
    let i = heap.len() - 1;
    while i > 0 {
        let p = (i - 1) / 2;
        if heap[p][0] >= heap[i][0] { break; }
        swap_at(heap, p, i);
        i = p;
    }
    heap
}

fn hpop_max(heap) {
    let n = heap.len();
    heap[0] = heap[n - 1];
    heap.truncate(n - 1);
    let i = 0;
    loop {
        let l = 2 * i + 1;
        let r = 2 * i + 2;
        let big = i;
        if l < heap.len() && heap[l][0] > heap[big][0] { big = l; }
        if r < heap.len() && heap[r][0] > heap[big][0] { big = r; }
        if big == i { break; }
        swap_at(heap, i, big);
        i = big;
    }
    heap
}

fn shown(heap) {
    let out = [];
    for e in heap { out.push(e[1]); }
    out
}

// Single-Threaded CPU — one ordering to arrive, another to run.
//
// Availability is a property of time; priority is a property of duration. A
// single sorted list cannot serve both, so the tasks are sorted by arrival and
// a cursor releases them into a heap ordered by duration. The idle case is
// worth noticing: rather than advancing the clock a tick at a time, it jumps
// straight to the next arrival — the number of interesting instants is bounded
// by the number of tasks, not by how large the timestamps are.

fn parse_tasks(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

fn labels(tasks, order) {
    let out = [];
    for i in order { out.push(`#${i} @${tasks[i][0]} takes ${tasks[i][1]}`); }
    out
}

fn trace(input) {
    let tasks = parse_tasks(input.tasks);
    let n = tasks.len();

    enter("getOrder", #{ n: N(n) });

    let idx = [];
    for i in 0..n { idx.push(i); }
    for i in 1..n {
        let j = i;
        while j > 0 && tasks[idx[j - 1]][0] > tasks[idx[j]][0] {
            swap_at(idx, j - 1, j);
            j -= 1;
        }
    }
    step("sort", "Sorted by availability, so a single forward cursor can release tasks as the clock passes them.",
        [ array("tasks, by availability", labels(tasks, idx)) ]);

    let h = [];
    step("init", "A min-heap keyed by processing time, with the index breaking ties.",
        [ heap("available tasks (shortest on top)", shown(h)) ]);

    let time = 0;
    set(#{ time: N(time) });
    step("clock", "The CPU clock.", [ text("cpu", [`time = ${time}`]) ]);

    let order = [];
    step("order", "The order tasks actually run in.", [ array("run order", order) ]);

    let i = 0;
    set(#{ i: N(i) });
    step("cursor", "The cursor into the availability-sorted list.",
        [ array("tasks, by availability", labels(tasks, idx)).ptr("i", i) ]);

    while order.len() < n {
        step("loop", `${order.len()} of ${n} tasks done; clock at ${time}.`,
            [ array("tasks, by availability", labels(tasks, idx)).ptr("i", i).done(before(i)),
              heap("available tasks (shortest on top)", shown(h)),
              text("cpu", [`time = ${time}`]) ]);

        while i < n && tasks[idx[i]][0] <= time {
            step("arrive", `Task #${idx[i]} became available at ${tasks[idx[i]][0]}, which is now in the past.`,
                [ array("tasks, by availability", labels(tasks, idx)).ptr("i", i).hl([i]) ]);
            h = hpush_min(h, tasks[idx[i]][1] * 1000 + idx[i], `#${idx[i]} (${tasks[idx[i]][1]})`);
            step("enqueue", `Into the heap. The key is its processing time, with the index appended so ties break by index automatically.`,
                [ heap("available tasks (shortest on top)", shown(h)) ]);
            i += 1;
            set(#{ i: N(i) });
        }

        if h.len() == 0 {
            step("idle", `Nothing is available at time ${time}, so the CPU is idle.`,
                [ heap("available tasks (shortest on top)", shown(h)),
                  text("cpu", [`time = ${time}`, "idle"]) ]);
            time = tasks[idx[i]][0];
            set(#{ time: N(time) });
            step("jump", `Jump straight to ${time}, when the next task arrives — there is nothing to decide in between.`,
                [ text("cpu", [`time = ${time}`]) ]);
            continue;
        }

        let t = h[0];
        h = hpop_min(h);
        let chosen = t[0] % 1000;
        let cost = t[0] / 1000;
        step("pick", `The shortest available task is #${chosen}, taking ${cost}.`,
            [ heap("available tasks (shortest on top)", shown(h)),
              text("cpu", [`time = ${time}`, `running #${chosen}`]) ]);

        time += cost;
        set(#{ time: N(time) });
        step("run", `It runs to completion — no pre-emption — so the clock reaches ${time}.`,
            [ text("cpu", [`time = ${time}`]) ]);

        order.push(chosen);
        log(`run #${chosen}, finishing at ${time}`);
        step("append", `Run order so far: ${join(order, ", ")}.`,
            [ array("run order", order) ]);
    }

    let answer = join(order, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Every task ran: ${answer}.`, [ array("run order", order) ], "return");
    leave(answer);
}
