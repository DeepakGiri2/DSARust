// Kth Largest Element — a min-heap of size k.
//
// The heap is kept at exactly k elements, and its smallest member is always
// the weakest of the current top k. Anything that cannot beat that member is
// evicted immediately, so the heap never grows and the answer sits on top at
// the end.

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
    let nums = input.nums;
    let k = input.k;
    let heap = [];

    enter("findKthLargest", #{ nums: A(nums), k: N(k) });
    set(#{ heap: A(heap) });
    step("init",
        `A min-heap that will never hold more than ${k} value(s). Its root is always the smallest of the best ${k} seen so far.`,
        [ array("nums", nums), heap("min-heap (root = smallest kept)", heap) ]);

    for i in 0..nums.len() {
        let x = nums[i];
        set(#{ i: N(i), x: N(x) });
        step("loop", `Consider nums[${i}] = ${x}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              heap("min-heap (root = smallest kept)", heap) ]);

        heap = heap_push(heap, x);
        set(#{ heap: A(heap) });
        log(`push ${x} -> heap size ${heap.len()}`);
        step("push", `Push ${x}. The heap holds ${heap.len()} value(s).`,
            [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)),
              heap("min-heap (root = smallest kept)", heap).pushed(true) ]);

        if heap.len() > k {
            step("check", `That is more than ${k}, so the weakest one no longer belongs in the top ${k}.`,
                [ array("nums", nums).ptr("i", i).done(before(i)),
                  heap("min-heap (root = smallest kept)", heap).bad([]) ]);
            let evicted = heap[0];
            heap = heap_pop(heap);
            set(#{ heap: A(heap) });
            log(`pop ${evicted}`);
            step("pop", `Drop ${evicted}, the smallest. Whatever is left is still the best ${k}.`,
                [ array("nums", nums).ptr("i", i).done(before(i)),
                  heap("min-heap (root = smallest kept)", heap).popped(evicted) ]);
        } else {
            step("check", `The heap holds ${heap.len()} value(s), still at most ${k} — nothing to evict.`,
                [ array("nums", nums).ptr("i", i).done(before(i)),
                  heap("min-heap (root = smallest kept)", heap) ]);
        }
    }

    let answer = heap[0];
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("return",
        `The heap holds exactly the ${k} largest values, and its root is the smallest of them — which is the ${k}th largest overall: ${answer}.`,
        [ array("nums", nums).done(seq(0, nums.len() - 1)),
          heap("min-heap (root = smallest kept)", heap).hl([]) ],
        "return");
    leave(`${answer}`);
}
