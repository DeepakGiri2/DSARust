fn parse_pairs(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

fn labels(ivs) {
    let out = [];
    for iv in ivs { out.push(`[${iv[0]},${iv[1]}]`); }
    out
}

fn sort_by_start(ivs) {
    for i in 1..ivs.len() {
        let j = i;
        while j > 0 && ivs[j - 1][0] > ivs[j][0] {
            swap_at(ivs, j - 1, j);
            j -= 1;
        }
    }
    ivs
}

// Meeting Rooms — sorting turns n² comparisons into n.
//
// The point of sorting by start time is not tidiness. Once the meetings are in
// order, the only thing that can clash with meeting i is meeting i-1: every
// earlier meeting starts no later, so if it ran long enough to reach meeting i
// it would already have overlapped meeting i-1 and the answer would be settled.

fn trace(input) {
    let ivs = parse_pairs(input.intervals);
    let n = ivs.len();

    enter("canAttendMeetings", #{ n: N(n) });

    ivs = sort_by_start(ivs);
    step("sort", "Sorted by start time. Now a clash can only be with the immediately preceding meeting.",
        [ array("meetings (sorted by start)", labels(ivs)) ]);

    let ok = true;
    for i in 1..n {
        step("loop", `Meeting ${i}: ${labels(ivs)[i]}.`,
            [ array("meetings (sorted by start)", labels(ivs)).ptr("i", i).done(before(i)) ]);

        let prev_end = ivs[i - 1][1];
        step("prev", `The previous meeting ends at ${prev_end}.`,
            [ array("meetings (sorted by start)", labels(ivs)).ptr("i", i).hl([i - 1, i]) ]);

        if ivs[i][0] < prev_end {
            ok = false;
            log(`${labels(ivs)[i - 1]} overlaps ${labels(ivs)[i]}`, "result");
            step("overlap", `This one starts at ${ivs[i][0]}, before ${prev_end} — the two overlap.`,
                [ array("meetings (sorted by start)", labels(ivs)).bad([i - 1, i]) ]);
            step("no", "So they cannot all be attended.", [ text("answer", ["false"]) ], "return");
            break;
        }

        step("overlap", `It starts at ${ivs[i][0]}, which is not before ${prev_end} — no clash.`,
            [ array("meetings (sorted by start)", labels(ivs)).hl([i - 1, i]) ]);
    }

    if ok {
        log("no overlaps", "result");
        step("yes", "Every adjacent pair is clear, and sorting guarantees no other pair could clash.",
            [ array("meetings (sorted by start)", labels(ivs)).done(seq(0, n - 1)) ], "return");
    }

    result(`${ok}`);
    leave(`${ok}`);
}
