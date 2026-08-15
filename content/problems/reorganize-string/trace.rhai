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

// Reorganize String — spend the most plentiful letter first, and park it.
//
// The letter with the most copies is the one that will eventually be forced
// into a repeat, so it must be placed at every opportunity. The "park it for
// one round" trick is what makes that safe: the letter just used is held out
// of the heap for exactly one step, so it cannot be chosen twice in a row, and
// it comes back immediately afterwards. If the heap empties with characters
// still unplaced, some letter outnumbered every gap and no arrangement exists.

// Key packs count first, then character, so the heap prefers the most
// plentiful letter and breaks ties alphabetically.
fn key_of(count, ch) {
    count * 100 + (25 - (ord(ch) - 97))
}

fn count_of(key) { key / 100 }

fn counts_view(counts) {
    let out = [];
    for e in counts { out.push(`${e[0]}: ${e[1]}`); }
    out
}

fn trace(input) {
    let s = input.s;
    let cs = chars(s);

    enter("reorganizeString", #{ n: N(cs.len()) });

    let counts = map_new();
    for c in cs { map_inc(counts, c, 1); }
    step("count", `How many of each letter: ${join(counts_view(counts), ", ")}.`,
        [ array("input", cs), kv("letter counts", counts) ]);

    let h = [];
    for e in counts { h = hpush_max(h, key_of(e[1], e[0]), `${e[0]}×${e[1]}`); }
    step("init", "A max-heap by remaining count, so the most plentiful letter is always on top.",
        [ heap("letters (most remaining on top)", shown(h)) ]);

    let out = [];
    step("out", "The rearranged string.", [ text("building", [join(out, "")]) ]);

    let prev_key = -1;
    let prev_ch = "";
    step("prev", "The letter used last round is parked here — out of the heap, so it cannot be picked again immediately.",
        [ text("parked", ["(nothing yet)"]) ]);

    while h.len() > 0 {
        step("loop", `${plural(h.len(), "letter")} available.`,
            [ heap("letters (most remaining on top)", shown(h)),
              text("building", [join(out, "")]) ]);

        let top = h[0];
        h = hpop_max(h);
        let ch = top[1].split("×")[0];
        let left = count_of(top[0]);
        step("pop", `${ch} has the most left (${left}), so it goes next — the letter in greatest supply is the one most likely to be stranded later.`,
            [ heap("letters (most remaining on top)", shown(h)) ]);

        out.push(ch);
        step("emit", `"${join(out, "")}".`, [ text("building", [join(out, "")]) ]);

        left -= 1;
        step("spend", `${left} copies of ${ch} remain.`,
            [ text("building", [join(out, "")]) ]);

        if prev_key >= 0 && count_of(prev_key) > 0 {
            step("restore", `${prev_ch} was parked last round and still has ${count_of(prev_key)} left.`,
                [ text("parked", [`${prev_ch} ×${count_of(prev_key)}`]) ]);
            h = hpush_max(h, prev_key, `${prev_ch}×${count_of(prev_key)}`);
            step("back", "Put it back in the heap — one round of exclusion is exactly enough to prevent a repeat.",
                [ heap("letters (most remaining on top)", shown(h)) ]);
        } else {
            step("restore", "Nothing parked, or the parked letter is exhausted.",
                [ text("parked", [if prev_key < 0 { "(nothing)" } else { `${prev_ch} ×0` }]) ]);
            step("back", "Nothing to put back.",
                [ heap("letters (most remaining on top)", shown(h)) ]);
        }

        prev_key = key_of(left, ch);
        prev_ch = ch;
        log(`place ${ch}; ${left} left`);
        step("hold", `Park ${ch} until next round.`,
            [ text("parked", [`${prev_ch} ×${left}`]) ]);
    }

    step("check", `${out.len()} of ${cs.len()} characters placed.`,
        [ text("building", [join(out, "")]) ]);

    if out.len() < cs.len() {
        log("no valid arrangement", "result");
        step("fail", "The heap emptied with characters left over, which can only happen when one letter outnumbers every available gap.",
            [ text("answer", ["(impossible)"]) ], "return");
        result("(impossible)");
        leave("(impossible)");
        return;
    }

    let answer = join(out, "");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `No two adjacent characters match: "${answer}".`,
        [ text("answer", [answer]) ], "return");
    leave(answer);
}
