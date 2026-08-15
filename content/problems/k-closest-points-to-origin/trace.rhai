// K Closest Points — a max-heap capped at k.
//
// The heap holds the best k found so far, worst at the root. That makes the
// decision for each new point a single comparison. Distances are compared
// squared: the square root is monotone, so it changes nothing but costs time.

fn hpush(heap, d, label) {
    heap.push([d, label]);
    let i = heap.len() - 1;
    while i > 0 {
        let p = (i - 1) / 2;
        if heap[p][0] >= heap[i][0] { break; }
        swap_at(heap, p, i);
        i = p;
    }
    heap
}

fn hpop(heap) {
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
    for e in heap { out.push(`${e[1]}(d${e[0]})`); }
    out
}

fn trace(input) {
    let pts = [];
    for pair in words(input.points) {
        let parts = pair.split(",");
        if parts.len() == 2 { pts.push([num(parts[0]), num(parts[1])]); }
    }
    let k = input.k;
    let heap = [];

    enter("kClosest", #{ k: N(k) });
    step("init", `A max-heap that never holds more than ${k} point(s): its root is the worst of the current best.`,
        [ heap(`heap (root = furthest kept)`, shown(heap)) ]);

    for i in 0..pts.len() {
        let p = pts[i];
        step("loop", `Point (${p[0]}, ${p[1]}).`,
            [ heap(`heap (root = furthest kept)`, shown(heap)) ]);

        let d = p[0] * p[0] + p[1] * p[1];
        set(#{ dist: N(d) });
        step("dist", `Squared distance ${p[0]}² + ${p[1]}² = ${d}. No square root needed — it would not change the order.`,
            [ heap(`heap (root = furthest kept)`, shown(heap)) ]);

        heap = hpush(heap, d, `${p[0]},${p[1]}`);
        step("push", `Push it; the heap holds ${heap.len()}.`,
            [ heap(`heap (root = furthest kept)`, shown(heap)).pushed(true) ]);

        if heap.len() > k {
            let worst = heap[0];
            step("check", `More than ${k} — the root, ${worst[1]} at distance ${worst[0]}, is the worst.`,
                [ heap(`heap (root = furthest kept)`, shown(heap)) ]);
            heap = hpop(heap);
            log(`evict ${worst[1]}`);
            step("pop", `Evict it.`,
                [ heap(`heap (root = furthest kept)`, shown(heap)).popped(`${worst[1]}`) ]);
        } else {
            step("check", `Still at most ${k} — nothing to evict.`,
                [ heap(`heap (root = furthest kept)`, shown(heap)) ]);
        }
    }

    let out = [];
    for e in heap { out.push(e[1]); }
    let answer = join(out, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `The ${k} closest point(s): ${answer}.`,
        [ heap(`heap (root = furthest kept)`, shown(heap)) ], "return");
    leave(answer);
}
