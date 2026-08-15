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

// Furthest Building — commit to the ladder, then take back the cheapest one.
//
// The right answer is "spend ladders on the largest climbs", but the largest
// climbs are not known while walking forwards. The fix is to decide
// optimistically and revise: give every climb a ladder, and whenever that
// exceeds the supply, reclaim the ladder from the smallest climb currently
// using one and pay for it in bricks. The min-heap makes "smallest currently
// using one" a single look, and paying bricks for the smallest is always the
// cheapest possible correction.

fn trace(input) {
    let heights = input.heights;
    let n = heights.len();
    let bricks = input.bricks;
    let ladders = input.ladders;

    enter("furthestBuilding", #{ bricks: N(bricks), ladders: N(ladders) });

    let h = [];
    step("init", "The climbs currently using a ladder, smallest on top.",
        [ bars("heights", heights),
          heap("climbs on ladders (smallest on top)", shown(h)),
          text("supplies", [`bricks: ${bricks}`, `ladders: ${ladders}`]) ]);

    let answer = n - 1;
    let reached = n - 1;
    for i in 0..(n - 1) {
        step("loop", `Standing on building ${i}.`,
            [ bars("heights", heights).ptr("i", i).done(before(i)),
              heap("climbs on ladders (smallest on top)", shown(h)),
              text("supplies", [`bricks: ${bricks}`, `ladders in use: ${h.len()} of ${ladders}`]) ]);

        let diff = heights[i + 1] - heights[i];
        step("diff", `The next building is ${heights[i + 1]}, a change of ${diff}.`,
            [ bars("heights", heights).ptr("i", i).hl([i, i + 1]) ]);

        if diff <= 0 {
            step("downhill", "Going down or level costs nothing at all.",
                [ bars("heights", heights).ptr("i", i).done(before(i + 1)) ]);
            continue;
        }

        h = hpush_min(h, diff, `${diff}`);
        step("ladder", `Give this climb a ladder for now — deciding optimistically is fine, because the decision can be revised.`,
            [ heap("climbs on ladders (smallest on top)", shown(h)) ]);

        if h.len() > ladders {
            step("tooMany", `${h.len()} climbs are on ladders but only ${ladders} exist.`,
                [ heap("climbs on ladders (smallest on top)", shown(h)) ]);
            let smallest = h[0];
            h = hpop_min(h);
            bricks -= smallest[0];
            set(#{ bricks: N(bricks) });
            log(`pay ${smallest[0]} bricks; ${bricks} left`);
            step("swap", `Reclaim the ladder from the smallest climb, ${smallest[0]}, and pay bricks for it instead — the cheapest possible way to free a ladder. ${bricks} bricks left.`,
                [ heap("climbs on ladders (smallest on top)", shown(h)),
                  text("supplies", [`bricks: ${bricks}`, `ladders in use: ${h.len()} of ${ladders}`]) ]);
        } else {
            step("tooMany", `${h.len()} of ${ladders} ladders in use — still within budget.`,
                [ heap("climbs on ladders (smallest on top)", shown(h)) ]);
        }

        if bricks < 0 {
            answer = i;
            log(`out of bricks at building ${i}`, "result");
            step("broke", `Bricks have gone negative, so that last climb was never affordable.`,
                [ bars("heights", heights).ptr("i", i).bad([i + 1]),
                  text("supplies", [`bricks: ${bricks}`]) ]);
            step("ret", `The furthest building reachable is ${i}.`,
                [ bars("heights", heights).hl(seq(0, i)) ], "return");
            reached = i;
            break;
        }
    }

    if reached == n - 1 && answer == n - 1 {
        log(`reached the end`, "result");
        step("end", `Every gap was paid for, so the last building, ${n - 1}, is reachable.`,
            [ bars("heights", heights).hl(seq(0, n - 1)),
              text("supplies", [`bricks left: ${bricks}`, `ladders in use: ${h.len()}`]) ], "return");
    }

    result(`${answer}`);
    leave(`${answer}`);
}
