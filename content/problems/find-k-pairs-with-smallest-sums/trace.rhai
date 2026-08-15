// Find K Pairs — a frontier over a matrix that is never built.
//
// Every pair is a cell of an n×m matrix of sums, sorted along both axes. The
// smallest cell is the top-left. Once a cell is taken, the only new candidate
// it makes reachable is the one to its right, because the one below it is
// already represented by its own row's frontier entry. So the heap holds at
// most one cell per row and stays small no matter how large the matrix is.

fn heap_items(entries, nums1, nums2) {
    let out = [];
    for e in entries { out.push(`${nums1[e[1]]}+${nums2[e[2]]}=${e[0]}`); }
    out
}

fn heap_insert(entries, e) {
    // A real priority queue; kept in order here so the picture reads as one.
    let i = 0;
    while i < entries.len() && entries[i][0] <= e[0] { i += 1; }
    entries.insert(i, e);
    entries
}

fn trace(input) {
    let nums1 = input.nums1;
    let nums2 = input.nums2;
    let k = input.k;

    enter("kSmallestPairs", #{ nums1: A(nums1), nums2: A(nums2), k: N(k) });

    let heapv = [];
    step("init", `A min-heap of candidate pairs. It will never hold more than ${nums1.len()} entries — one per row of the sum matrix.`,
        [ array("nums1", nums1), array("nums2", nums2) ]);

    let limit = if nums1.len() < k { nums1.len() } else { k };
    step("seed", `Seed it with the first ${limit} pairs of column 0. Every row's smallest sum uses nums2[0], so this is the whole starting frontier.`,
        [ array("nums1", nums1).window(0, limit - 1), array("nums2", nums2).hl([0]) ]);

    for i in 0..limit {
        heapv = heap_insert(heapv, [nums1[i] + nums2[0], i, 0]);
        step("push", `Push (${nums1[i]}, ${nums2[0]}) with sum ${nums1[i] + nums2[0]}.`,
            [ array("nums1", nums1).ptr("i", i).hl([i]),
              heap("heap — candidate pairs by sum", heap_items(heapv, nums1, nums2)).pushed(true) ]);
    }

    let out = [];
    step("out", "Now pop k times. Each pop is the smallest sum still reachable.",
        [ heap("heap — candidate pairs by sum", heap_items(heapv, nums1, nums2)) ]);

    while !heapv.is_empty() && out.len() < k {
        step("loop", `${out.len()} of ${k} pairs found.`,
            [ heap("heap — candidate pairs by sum", heap_items(heapv, nums1, nums2)) ]);

        let p = heapv[0];
        heapv.remove(0);
        set(#{ i: N(p[1]), j: N(p[2]), sum: N(p[0]) });
        step("pop", `Pop the smallest: (${nums1[p[1]]}, ${nums2[p[2]]}) summing to ${p[0]}.`,
            [ array("nums1", nums1).hl([p[1]]), array("nums2", nums2).hl([p[2]]),
              heap("heap — candidate pairs by sum", heap_items(heapv, nums1, nums2)).popped(true) ]);

        out.push(`${nums1[p[1]]} ${nums2[p[2]]}`);
        log(`pair (${nums1[p[1]]}, ${nums2[p[2]]})`, "result");
        step("take", `Take it. Answer so far: ${join(out, " | ")}.`,
            [ text("pairs taken", out) ]);

        if p[2] + 1 < nums2.len() {
            step("more", `Row ${p[1]} has more to give: the next cell to the right is (${nums1[p[1]]}, ${nums2[p[2] + 1]}).`,
                [ array("nums2", nums2).ptr("j", p[2] + 1).hl([p[2] + 1]) ]);
            heapv = heap_insert(heapv, [nums1[p[1]] + nums2[p[2] + 1], p[1], p[2] + 1]);
            step("next", `Push it. One pop, at most one push — the heap stays bounded by the number of rows.`,
                [ heap("heap — candidate pairs by sum", heap_items(heapv, nums1, nums2)).pushed(true) ]);
        } else {
            step("more", `Row ${p[1]} is exhausted — nothing to push back.`,
                [ heap("heap — candidate pairs by sum", heap_items(heapv, nums1, nums2)) ]);
        }
    }

    let answer = join(out, " | ");
    log(answer, "result");
    result(answer);
    step("ret", `The ${plural(out.len(), "smallest pair")}, without ever forming ${nums1.len() * nums2.len()} sums.`,
        [ text("pairs taken", out) ], "return");
    leave(answer);
}
