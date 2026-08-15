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

// IPO — affordability is monotone, so nothing ever has to be reconsidered.
//
// Capital only grows, so once a project is affordable it stays affordable
// forever. That is what makes the one-way cursor legitimate: projects are
// released into the pool as capital passes their requirement and never need
// removing. And taking the most profitable affordable project is safe for the
// same reason — a larger gain now cannot close off any option that a smaller
// one would have kept open.

fn labels(profits, capital, order) {
    let out = [];
    for i in order { out.push(`#${i}: needs ${capital[i]}, pays ${profits[i]}`); }
    out
}

fn trace(input) {
    let k = input.k;
    let w = input.w;
    let profits = input.profits;
    let capital = input.capital;
    let n = profits.len();

    enter("findMaximizedCapital", #{ k: N(k), w: N(w) });

    let idx = [];
    for i in 0..n { idx.push(i); }
    for i in 1..n {
        let j = i;
        while j > 0 && capital[idx[j - 1]] > capital[idx[j]] {
            swap_at(idx, j - 1, j);
            j -= 1;
        }
    }
    step("sort", "Sorted by capital requirement. Since capital only ever rises, a single forward cursor can release projects as they become affordable.",
        [ array("projects, by capital needed", labels(profits, capital, idx)) ]);

    let h = [];
    step("init", "A max-heap of the profits of everything currently affordable.",
        [ heap("affordable projects (most profitable on top)", shown(h)),
          text("capital", [`${w}`]) ]);

    let i = 0;
    set(#{ i: N(i) });
    step("cursor", "The cursor into the sorted list.",
        [ array("projects, by capital needed", labels(profits, capital, idx)).ptr("i", i) ]);

    for round in 0..k {
        step("loop", `Round ${round + 1} of ${k}; capital is ${w}.`,
            [ array("projects, by capital needed", labels(profits, capital, idx))
                .ptr("i", i).done(before(i)),
              heap("affordable projects (most profitable on top)", shown(h)),
              text("capital", [`${w}`]) ]);

        while i < n && capital[idx[i]] <= w {
            step("afford", `Project #${idx[i]} needs ${capital[idx[i]]}, which ${w} covers.`,
                [ array("projects, by capital needed", labels(profits, capital, idx))
                    .ptr("i", i).hl([i]) ]);
            h = hpush_max(h, profits[idx[i]], `#${idx[i]} (+${profits[idx[i]]})`);
            step("offer", "Add it to the pool — and it can never leave, because capital never falls.",
                [ heap("affordable projects (most profitable on top)", shown(h)) ]);
            i += 1;
            set(#{ i: N(i) });
        }

        if h.len() == 0 {
            step("none", `Nothing is affordable with ${w}, and nothing ever will be — so stop early.`,
                [ heap("affordable projects (most profitable on top)", shown(h)),
                  text("capital", [`${w}`]) ]);
            break;
        }

        let top = h[0];
        h = hpop_max(h);
        w += top[0];
        set(#{ w: N(w) });
        log(`take ${top[1]}, capital ${w}`);
        step("take", `Take the most profitable affordable project, ${top[1]}. Capital becomes ${w} — which can only widen what is affordable next round.`,
            [ heap("affordable projects (most profitable on top)", shown(h)),
              text("capital", [`${w}`]) ]);
    }

    log(`answer ${w}`, "result");
    result(`${w}`);
    step("ret", `Final capital: ${w}.`, [ text("capital", [`${w}`]) ], "return");
    leave(`${w}`);
}
