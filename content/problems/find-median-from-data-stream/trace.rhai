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

// Find Median From Data Stream — two heaps facing each other.
//
// The median is defined by position, so keeping the values split at that
// position is the natural state to maintain. Orient each half so the value
// nearest the middle is on top — largest of the low half, smallest of the high
// half — and the median is one peek away, or the average of two.
//
// The insertion looks roundabout: push low, move the top across, maybe move
// one back. That sequence is what guarantees correctness without comparing the
// new value to anything: routing it through the low half places it correctly
// even if it belongs above, and the rebalance fixes only the sizes.

fn half_str(sum) {
    if sum % 2 == 0 { return `${sum / 2}`; }
    if sum > 0 { return `${sum / 2}.5`; }
    `-${(-sum) / 2}.5`
}

fn trace(input) {
    let low = [];   // max-heap: keys are the values themselves
    let high = [];  // min-heap
    let out = [];

    enter("MedianFinder", #{});
    step("init", "Two heaps: the lower half with its largest on top, the upper half with its smallest on top. The boundary between them is exactly where the median lives.",
        [ heap("low half (max-heap)", shown(low)), heap("high half (min-heap)", shown(high)) ]);

    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() == 0 { continue; }

        if parts[0] == "add" {
            let x = num(parts[1]);
            enter(`addNum(${x})`, #{ x: N(x) });

            low = hpush_max(low, x, `${x}`);
            step("pushLow", `Push ${x} into the low half — unconditionally, without comparing it to anything.`,
                [ heap("low half (max-heap)", shown(low)), heap("high half (min-heap)", shown(high)) ]);

            let top = low[0];
            low = hpop_max(low);
            high = hpush_min(high, num(top[1]), top[1]);
            step("shift", `Move the low half's largest, ${top[1]}, across to the high half. That is what puts the new value on the correct side even when it belongs above the median.`,
                [ heap("low half (max-heap)", shown(low)), heap("high half (min-heap)", shown(high)) ]);

            if high.len() > low.len() {
                step("rebalance", `The high half now holds ${high.len()} against ${low.len()} — too many.`,
                    [ heap("low half (max-heap)", shown(low)),
                      heap("high half (min-heap)", shown(high)) ]);
                let back = high[0];
                high = hpop_min(high);
                low = hpush_max(low, num(back[1]), back[1]);
                log(`add ${x}; halves ${low.len()}/${high.len()}`);
                step("moveBack", `Send ${back[1]} back, so the low half is never smaller than the high half. Only the sizes moved — the ordering was already right.`,
                    [ heap("low half (max-heap)", shown(low)),
                      heap("high half (min-heap)", shown(high)) ], "return");
            } else {
                log(`add ${x}; halves ${low.len()}/${high.len()}`);
                step("rebalance", `Sizes are ${low.len()} and ${high.len()} — already balanced, nothing to move.`,
                    [ heap("low half (max-heap)", shown(low)),
                      heap("high half (min-heap)", shown(high)) ], "return");
            }
            leave("");
            continue;
        }

        enter("findMedian()", #{});
        if low.len() > high.len() {
            let m = low[0][1];
            out.push(m);
            log(`median ${m}`, "result");
            step("odd", `${low.len() + high.len()} values is odd, so the extra one sits at the top of the low half.`,
                [ heap("low half (max-heap)", shown(low)),
                  heap("high half (min-heap)", shown(high)) ]);
            step("retOdd", `The median is ${m}.`, [ text("output", out) ], "return");
            leave(m);
        } else {
            let a = num(low[0][1]);
            let b = num(high[0][1]);
            let m = half_str(a + b);
            out.push(m);
            log(`median ${m}`, "result");
            step("odd", `${low.len() + high.len()} values is even, so the median straddles the boundary.`,
                [ heap("low half (max-heap)", shown(low)),
                  heap("high half (min-heap)", shown(high)) ]);
            step("retEven", `Average the two values on either side: (${a} + ${b}) / 2 = ${m}.`,
                [ text("output", out) ], "return");
            leave(m);
        }
    }

    let answer = join(out, " ");
    result(answer);
    leave(answer);
}
