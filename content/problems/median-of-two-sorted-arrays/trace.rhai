// Median of Two Sorted Arrays — search for a cut, not for a number.
//
// The median is defined by a *split*: half the values below it, half above. So
// look for the split directly. Cut each array once so the two left pieces
// together hold exactly half of everything; the cut is right when nothing on
// either left side exceeds anything on either right side. Choosing where to cut
// the first array fixes the second, so there is only one degree of freedom —
// and it lives in the shorter array.

// Sentinels stand in for ±infinity so an index that falls off either end needs
// no special case. Rhai script functions do not see outer-scope constants, so
// the values are written out where they are used.
fn edge_lo(arr, i) { if i < 0 { -1000000 } else { arr[i] } }
fn edge_hi(arr, i) { if i >= arr.len() { 1000000 } else { arr[i] } }
fn show_edge(v) {
    if v == -1000000 { "-∞" } else if v == 1000000 { "+∞" } else { `${v}` }
}

fn half_str(sum) {
    if sum % 2 == 0 { return `${sum / 2}`; }
    if sum > 0 { return `${sum / 2}.5`; }
    `-${(-sum) / 2}.5`
}

fn trace(input) {
    let a = input.nums1;
    let b = input.nums2;

    enter("findMedianSortedArrays", #{});
    if a.len() > b.len() {
        let t = a;
        a = b;
        b = t;
        step("swap", "Swap so the binary search runs over the shorter array — that is what makes the cost log of the *smaller* length.",
            [ array("shorter (a)", a), array("longer (b)", b) ]);
    } else {
        step("swap", "The first array is already the shorter one, so no swap is needed.",
            [ array("shorter (a)", a), array("longer (b)", b) ]);
    }

    let m = a.len();
    let n = b.len();
    set(#{ m: N(m), n: N(n) });
    step("sizes", `${m} value(s) in the short array, ${n} in the long one.`,
        [ array("shorter (a)", a), array("longer (b)", b) ]);

    let half = (m + n + 1) / 2;
    set(#{ half: N(half) });
    step("half", `A correct cut leaves exactly ${half} value(s) on the left in total. Rounding up puts the extra one on the left, which is what makes the odd case read straight off the left edges.`,
        [ text("target", [`total values: ${m + n}`, `must be on the left: ${half}`]) ]);

    let lo = 0;
    let hi = m;
    set(#{ lo: N(lo), hi: N(hi) });
    step("bounds", `The cut in a can be anywhere from 0 (take nothing) to ${m} (take all of it).`,
        [ array("shorter (a)", a), array("longer (b)", b) ]);

    let answer = "";
    while lo <= hi {
        step("loop", `Cut positions ${lo}..${hi} are still possible for a.`,
            [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);

        let i = lo + (hi - lo) / 2;
        set(#{ i: N(i) });
        step("mid", `Try cutting a after ${plural(i, "value")}.`,
            [ array("shorter (a)", a).ptr("cut", i).done(before(i)),
              array("longer (b)", b) ]);

        let j = half - i;
        set(#{ j: N(j) });
        step("partner", `Then b must give up the other ${j} — the second cut is not a choice, it is forced by the count.`,
            [ array("shorter (a)", a).ptr("cut", i).done(before(i)),
              array("longer (b)", b).ptr("cut", j).done(before(j)) ]);

        let a_left = edge_lo(a, i - 1);
        let a_right = edge_hi(a, i);
        let b_left = edge_lo(b, j - 1);
        let b_right = edge_hi(b, j);
        step("edges", `Only the four values touching the cuts matter: ${show_edge(a_left)} | ${show_edge(a_right)} in a, ${show_edge(b_left)} | ${show_edge(b_right)} in b. Off the end counts as infinity, which removes every special case.`,
            [ array("shorter (a)", a).ptr("cut", i).done(before(i)),
              array("longer (b)", b).ptr("cut", j).done(before(j)),
              text("cut edges", [`left of a: ${show_edge(a_left)}   right of a: ${show_edge(a_right)}`,
                                 `left of b: ${show_edge(b_left)}   right of b: ${show_edge(b_right)}`]) ]);

        if a_left <= b_right && b_left <= a_right {
            step("check", `${show_edge(a_left)} ≤ ${show_edge(b_right)} and ${show_edge(b_left)} ≤ ${show_edge(a_right)}: nothing on the left exceeds anything on the right, so this cut really does separate the lower half from the upper half.`,
                [ array("shorter (a)", a).ptr("cut", i).done(before(i)),
                  array("longer (b)", b).ptr("cut", j).done(before(j)) ]);

            let max_left = if a_left > b_left { a_left } else { b_left };
            let min_right = if a_right < b_right { a_right } else { b_right };

            if (m + n) % 2 == 1 {
                step("odd", `${m + n} values is odd, so the median is a single element — the largest one on the left.`,
                    [ text("cut edges", [`largest on the left: ${show_edge(max_left)}`]) ]);
                answer = `${max_left}`;
                log(`median ${answer}`, "result");
                step("retOdd", `The median is ${answer}.`,
                    [ array("shorter (a)", a).ptr("cut", i),
                      array("longer (b)", b).ptr("cut", j),
                      text("answer", [`median = ${answer}`]) ], "return");
            } else {
                step("odd", `${m + n} values is even, so the median straddles the cut.`,
                    [ text("cut edges", [`largest on the left: ${show_edge(max_left)}`,
                                         `smallest on the right: ${show_edge(min_right)}`]) ]);
                answer = half_str(max_left + min_right);
                log(`median ${answer}`, "result");
                step("retEven", `Average the two values touching the cut: (${max_left} + ${min_right}) / 2 = ${answer}.`,
                    [ array("shorter (a)", a).ptr("cut", i),
                      array("longer (b)", b).ptr("cut", j),
                      text("answer", [`median = ${answer}`]) ], "return");
            }
            break;
        }

        if a_left > b_right {
            step("check", `${show_edge(a_left)} > ${show_edge(b_right)}: a value on a's left is bigger than one on b's right, so the cut is wrong.`,
                [ array("shorter (a)", a).ptr("cut", i).bad([i - 1]),
                  array("longer (b)", b).ptr("cut", j).bad([j]) ]);
            step("tooBig", "a is giving away too many values.",
                [ array("shorter (a)", a).ptr("cut", i) ]);
            hi = i - 1;
            set(#{ hi: N(hi) });
            step("shrink", `Cut a earlier: hi = ${hi}.`,
                [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);
        } else {
            step("check", `${show_edge(b_left)} > ${show_edge(a_right)}: a value on b's left is bigger than one on a's right, so the cut is wrong the other way.`,
                [ array("shorter (a)", a).ptr("cut", i).bad([i]),
                  array("longer (b)", b).ptr("cut", j).bad([j - 1]) ]);
            step("tooSmall", "a is not giving away enough.",
                [ array("shorter (a)", a).ptr("cut", i) ]);
            lo = i + 1;
            set(#{ lo: N(lo) });
            step("grow", `Cut a later: lo = ${lo}.`,
                [ text("search range", [`lo = ${lo}`, `hi = ${hi}`]) ]);
        }
    }

    result(answer);
    leave(answer);
}
