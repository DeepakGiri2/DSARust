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

// Car Pooling — the car only moves forwards, and the heap exploits that.
//
// Because the journey is one-way, "has this group got off yet?" is answered by
// comparing their destination with the current position — no re-checking, no
// going back. Sorting by pickup makes positions arrive in order, and the
// min-heap of destinations means the next group to leave is always on top. Each
// trip is pushed once and popped once.

fn parse_trips(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 3 { out.push([num(w[0]), num(w[1]), num(w[2])]); }
    }
    out
}

fn labels(trips) {
    let out = [];
    for t in trips { out.push(`${t[0]}p ${t[1]}→${t[2]}`); }
    out
}

fn trace(input) {
    let trips = parse_trips(input.trips);
    let capacity = input.capacity;
    let n = trips.len();

    enter("carPooling", #{ capacity: N(capacity) });

    for i in 1..n {
        let j = i;
        while j > 0 && trips[j - 1][1] > trips[j][1] {
            swap_at(trips, j - 1, j);
            j -= 1;
        }
    }
    step("sort", "Sorted by pickup point, so the car's position only ever moves forwards as the loop runs.",
        [ array("trips (passengers, from → to)", labels(trips)) ]);

    let h = [];
    step("init", "A min-heap of drop-off points for the groups currently aboard.",
        [ heap("aboard (earliest drop-off on top)", shown(h)) ]);

    let onboard = 0;
    set(#{ onboard: N(onboard) });
    step("onboard", "Passengers currently in the car.",
        [ text("car", [`aboard: ${onboard} of ${capacity}`]) ]);

    let ok = true;
    for k in 0..n {
        let t = trips[k];
        step("loop", `Trip ${k}: ${t[0]} passenger(s) from ${t[1]} to ${t[2]}. The car is now at ${t[1]}.`,
            [ array("trips (passengers, from → to)", labels(trips)).ptr("k", k).done(before(k)),
              heap("aboard (earliest drop-off on top)", shown(h)),
              text("car", [`position: ${t[1]}`, `aboard: ${onboard} of ${capacity}`]) ]);

        while h.len() > 0 && h[0][0] <= t[1] {
            let top = h[0];
            step("dropoff", `The earliest drop-off is at ${top[0]}, which is at or behind ${t[1]} — and the car never reverses, so those seats are free.`,
                [ heap("aboard (earliest drop-off on top)", shown(h)) ]);
            h = hpop_min(h);
            let left = num(top[1].split("x")[0]);
            onboard -= left;
            set(#{ onboard: N(onboard) });
            log(`${left} passenger(s) leave at ${top[0]}`);
            step("leave", `${plural(left, "passenger")} get out; ${onboard} remain.`,
                [ heap("aboard (earliest drop-off on top)", shown(h)),
                  text("car", [`aboard: ${onboard} of ${capacity}`]) ]);
        }

        onboard += t[0];
        set(#{ onboard: N(onboard) });
        step("board", `${plural(t[0], "passenger")} get in: ${onboard} aboard.`,
            [ text("car", [`aboard: ${onboard} of ${capacity}`]) ]);

        if onboard > capacity {
            ok = false;
            log(`${onboard} exceeds capacity ${capacity}`, "result");
            step("over", `${onboard} is more than the car holds (${capacity}).`,
                [ array("trips (passengers, from → to)", labels(trips)).bad([k]),
                  text("car", [`aboard: ${onboard} of ${capacity}`]) ]);
            step("no", "So the schedule is impossible.", [ text("answer", ["false"]) ], "return");
            break;
        }

        h = hpush_min(h, t[2], `${t[0]}x@${t[2]}`);
        step("push", `Remember that they leave at ${t[2]}.`,
            [ heap("aboard (earliest drop-off on top)", shown(h)) ]);
    }

    if ok {
        log("capacity never exceeded", "result");
        step("yes", `The car never held more than ${capacity}.`, [ text("answer", ["true"]) ], "return");
    }

    result(`${ok}`);
    leave(`${ok}`);
}
