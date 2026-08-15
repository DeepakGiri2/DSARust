// Kth Largest in a Stream — a min-heap capped at k.
//
// The class has to answer after *every* add, so re-sorting is out. Holding
// exactly the k largest values means the answer is always the root, and each
// add costs one push and at most one pop.

fn heap_push(heap, v) {
    heap.push(v);
    let i = heap.len() - 1;
    while i > 0 {
        let parent = (i - 1) / 2;
        if heap[parent] <= heap[i] { break; }
        swap_at(heap, parent, i);
        i = parent;
    }
    heap
}

fn heap_pop(heap) {
    let n = heap.len();
    if n == 0 { return heap; }
    heap[0] = heap[n - 1];
    heap.truncate(n - 1);
    let i = 0;
    loop {
        let l = 2 * i + 1;
        let r = 2 * i + 2;
        let small = i;
        if l < heap.len() && heap[l] < heap[small] { small = l; }
        if r < heap.len() && heap[r] < heap[small] { small = r; }
        if small == i { break; }
        swap_at(heap, i, small);
        i = small;
    }
    heap
}

fn trace(input) {
    let k = input.k;
    let nums = input.nums;
    let adds = input.adds;
    let heap = [];
    let answers = [];

    enter("KthLargest", #{ k: N(k) });
    step("ctor", `The stream must always report the ${k}th largest value seen so far.`,
        [ array("initial", nums), array("adds", adds) ]);
    set(#{ heap: A(heap) });
    step("init", `A min-heap that never grows past ${k} entries.`,
        [ heap(`min-heap (at most ${k})`, heap) ]);

    for i in 0..nums.len() {
        heap = heap_push(heap, nums[i]);
        if heap.len() > k { heap = heap_pop(heap); }
        set(#{ heap: A(heap) });
        step("ctorLoop", `Seed with ${nums[i]}; the heap holds ${heap.len()}.`,
            [ array("initial", nums).ptr("i", i).hl([i]).done(before(i)),
              heap(`min-heap (at most ${k})`, heap).pushed(true) ]);
    }

    for i in 0..adds.len() {
        let v = adds[i];
        set(#{ val: N(v) });
        step("add", `add(${v}).`,
            [ array("adds", adds).ptr("i", i).hl([i]).done(before(i)),
              heap(`min-heap (at most ${k})`, heap) ]);

        heap = heap_push(heap, v);
        set(#{ heap: A(heap) });
        step("push", `Push it — the heap now holds ${heap.len()}.`,
            [ array("adds", adds).ptr("i", i).done(before(i)),
              heap(`min-heap (at most ${k})`, heap).pushed(true) ]);

        if heap.len() > k {
            let out = heap[0];
            step("check", `That is more than ${k}, so the smallest kept value no longer belongs in the top ${k}.`,
                [ heap(`min-heap (at most ${k})`, heap) ]);
            heap = heap_pop(heap);
            set(#{ heap: A(heap) });
            log(`evict ${out}`);
            step("pop", `Drop ${out}.`,
                [ heap(`min-heap (at most ${k})`, heap).popped(out) ]);
        } else {
            step("check", `Still at most ${k} entries — nothing to evict.`,
                [ heap(`min-heap (at most ${k})`, heap) ]);
        }

        answers.push(heap[0]);
        log(`add(${v}) -> ${heap[0]}`, "result");
        step("return", `The root is the ${k}th largest: ${heap[0]}.`,
            [ array("adds", adds).ptr("i", i).done(before(i + 1)),
              heap(`min-heap (at most ${k})`, heap),
              array("answers", answers).hl([answers.len() - 1]) ]);
    }

    let out = join(answers, " ");
    result(out);
    step("return", `Every add answered in O(log k): ${out}.`,
        [ array("answers", answers).hl(seq(0, answers.len() - 1)) ], "return");
    leave(out);
}
