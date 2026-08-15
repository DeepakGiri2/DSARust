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

// Interval List Intersections — one formula, and a safe way to advance.
//
// max(starts) to min(ends) is the intersection whenever there is one, and
// comes out backwards — lo greater than hi — exactly when there is not. So no
// separate overlap test is needed beyond comparing the two.
//
// Advancing is the other half. Since each list is sorted and internally
// disjoint, the interval that ends first cannot reach anything further along
// the other list either, so it can be discarded with nothing left to check.

fn trace(input) {
    let a = parse_pairs(input.a);
    let b = parse_pairs(input.b);

    enter("intervalIntersection", #{});

    let res = [];
    step("init", "The intersections found.",
        [ array("list A", labels(a)), array("list B", labels(b)), text("intersections", res) ]);

    let i = 0;
    let j = 0;
    set(#{ i: N(i), j: N(j) });
    step("cursors", "One cursor per list; neither ever goes backwards.",
        [ array("list A", labels(a)).ptr("i", i), array("list B", labels(b)).ptr("j", j) ]);

    while i < a.len() && j < b.len() {
        step("loop", `Comparing ${labels(a)[i]} with ${labels(b)[j]}.`,
            [ array("list A", labels(a)).ptr("i", i).hl([i]).done(before(i)),
              array("list B", labels(b)).ptr("j", j).hl([j]).done(before(j)) ]);

        let lo = if a[i][0] > b[j][0] { a[i][0] } else { b[j][0] };
        step("lo", `An overlap could only start at the later of the two starts: max(${a[i][0]}, ${b[j][0]}) = ${lo}.`,
            [ text("candidate", [`lo = ${lo}`]) ]);

        let hi = if a[i][1] < b[j][1] { a[i][1] } else { b[j][1] };
        step("hi", `And could only end at the earlier of the two ends: min(${a[i][1]}, ${b[j][1]}) = ${hi}.`,
            [ text("candidate", [`lo = ${lo}`, `hi = ${hi}`]) ]);

        if lo <= hi {
            step("overlap", `${lo} ≤ ${hi}, so that range is real.`,
                [ array("list A", labels(a)).hl([i]), array("list B", labels(b)).hl([j]) ]);
            res.push(`${lo}-${hi}`);
            log(`intersection ${lo}-${hi}`);
            step("record", `Record [${lo},${hi}].`, [ text("intersections", res) ]);
        } else {
            step("overlap", `${lo} > ${hi} — the range came out backwards, which is exactly what "no overlap" looks like. No separate test needed.`,
                [ array("list A", labels(a)).ptr("i", i), array("list B", labels(b)).ptr("j", j) ]);
            step("record", "Nothing to record.", [ text("intersections", res) ]);
        }

        if a[i][1] < b[j][1] {
            step("advanceA", `${labels(a)[i]} ends first. List B is sorted and disjoint, so nothing later in B can reach back to it — drop it.`,
                [ array("list A", labels(a)).ptr("i", i).bad([i]) ]);
            i += 1;
            set(#{ i: N(i) });
        } else {
            step("advanceB", `${labels(b)[j]} ends first (or at the same time), so it is the one that can be discarded.`,
                [ array("list B", labels(b)).ptr("j", j).bad([j]) ]);
            j += 1;
            set(#{ j: N(j) });
        }
    }

    let answer = if res.len() == 0 { "(none)" } else { join(res, " ") };
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(res.len(), "intersection")}: ${answer}.`,
        [ text("intersections", res) ], "return");
    leave(answer);
}
