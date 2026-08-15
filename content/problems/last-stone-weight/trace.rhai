// Last Stone Weight — a max-heap answers "the two heaviest" repeatedly.
//
// Sorting once is not enough, because the smashed remainder has to be inserted
// back in order. A heap does both ends of that in O(log n).

fn heap_push(heap, v) {
    heap.push(v);
    let i = heap.len() - 1;
    while i > 0 {
        let parent = (i - 1) / 2;
        if heap[parent] >= heap[i] { break; }
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
        let big = i;
        if l < heap.len() && heap[l] > heap[big] { big = l; }
        if r < heap.len() && heap[r] > heap[big] { big = r; }
        if big == i { break; }
        swap_at(heap, i, big);
        i = big;
    }
    heap
}

fn trace(input) {
    let stones = input.stones;
    let heap = [];

    enter("lastStoneWeight", #{ stones: A(stones) });
    step("init", "A max-heap keeps the heaviest stone at the root.",
        [ array("stones", stones), heap("max-heap (root = heaviest)", heap) ]);

    for i in 0..stones.len() {
        heap = heap_push(heap, stones[i]);
        set(#{ heap: A(heap) });
        step("build", `Add ${stones[i]}. The heaviest so far is ${heap[0]}.`,
            [ array("stones", stones).ptr("i", i).hl([i]).done(before(i)),
              heap("max-heap (root = heaviest)", heap).pushed(true) ]);
    }

    while heap.len() > 1 {
        step("loop", `${heap.len()} stone(s) left, so there is still a smash to do.`,
            [ heap("max-heap (root = heaviest)", heap) ]);

        let a = heap[0];
        heap = heap_pop(heap);
        set(#{ y: N(a), heap: A(heap) });
        step("pop1", `The heaviest is ${a}.`,
            [ heap("max-heap (root = heaviest)", heap).popped(a) ]);

        let b = heap[0];
        heap = heap_pop(heap);
        set(#{ x: N(b), heap: A(heap) });
        step("pop2", `The next heaviest is ${b}.`,
            [ heap("max-heap (root = heaviest)", heap).popped(b) ]);

        if a != b {
            step("check", `${a} and ${b} differ, so a stone of ${a} − ${b} = ${a - b} survives.`,
                [ heap("max-heap (root = heaviest)", heap) ]);
            heap = heap_push(heap, a - b);
            set(#{ heap: A(heap) });
            log(`smash ${a} and ${b} -> ${a - b}`);
            step("push", `Put ${a - b} back into the heap, which re-sorts itself.`,
                [ heap("max-heap (root = heaviest)", heap).pushed(true) ]);
        } else {
            log(`smash ${a} and ${b} -> both destroyed`);
            step("check", `${a} and ${b} are equal, so both are destroyed and nothing goes back.`,
                [ heap("max-heap (root = heaviest)", heap) ]);
        }
    }

    let answer = if heap.len() == 0 { 0 } else { heap[0] };
    if heap.len() == 0 {
        step("empty", "Every stone was destroyed.", [ heap("max-heap (root = heaviest)", heap) ]);
    } else {
        step("empty", `One stone remains: ${answer}.`,
            [ heap("max-heap (root = heaviest)", heap).hl([]) ]);
    }
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("return", `The last stone weighs ${answer}.`,
        [ heap("max-heap (root = heaviest)", heap) ], "return");
    leave(`${answer}`);
}
