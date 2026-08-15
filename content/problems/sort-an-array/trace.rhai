// Sort an Array — merge sort, where all the work is in the merge.
//
// Splitting is free: the midpoint is arithmetic, and the two halves are not
// examined. Everything happens on the way back up, and the merge is linear for
// one reason — if both halves are sorted, the smaller of their two front
// elements is the smallest value remaining anywhere, so choosing the next
// output value is a single comparison rather than a search.

fn merge(a, b) {
    enter(`merge(${a.len()} + ${b.len()})`);

    let out = [];
    step("init", `Merging [${join(a, ", ")}] with [${join(b, ", ")}].`,
        [ array("left half", a), array("right half", b), array("merged", out) ]);

    let i = 0;
    let j = 0;
    step("cursors", "One cursor per half, both at the front.",
        [ array("left half", a).ptr("i", i), array("right half", b).ptr("j", j) ]);

    while i < a.len() && j < b.len() {
        step("loop", "Both halves still have values.",
            [ array("left half", a).ptr("i", i).done(before(i)),
              array("right half", b).ptr("j", j).done(before(j)),
              array("merged", out) ]);

        if a[i] <= b[j] {
            step("cmp", `${a[i]} ≤ ${b[j]}. Since both halves are sorted, ${a[i]} is the smallest value left anywhere — no searching needed.`,
                [ array("left half", a).ptr("i", i).hl([i]),
                  array("right half", b).ptr("j", j).hl([j]) ]);
            out.push(a[i]);
            i += 1;
            step("takeA", `Take it: [${join(out, ", ")}].`,
                [ array("merged", out).hl([out.len() - 1]) ]);
        } else {
            step("cmp", `${b[j]} < ${a[i]}, so the right half supplies the next value.`,
                [ array("left half", a).ptr("i", i).hl([i]),
                  array("right half", b).ptr("j", j).hl([j]) ]);
            out.push(b[j]);
            j += 1;
            step("takeB", `Take it: [${join(out, ", ")}].`,
                [ array("merged", out).hl([out.len() - 1]) ]);
        }
    }

    while i < a.len() {
        out.push(a[i]);
        i += 1;
    }
    step("drainA", `The right half is exhausted, so whatever is left on the left is already in order and can be copied straight across.`,
        [ array("left half", a), array("merged", out) ]);

    while j < b.len() {
        out.push(b[j]);
        j += 1;
    }
    step("drainB", "Likewise for the right half.",
        [ array("right half", b), array("merged", out) ]);

    log(`merged into [${join(out, ", ")}]`);
    step("ret", `Merged: [${join(out, ", ")}].`, [ array("merged", out) ], "return");
    leave(`${join(out, ",")}`);
    out
}

fn msort(nums) {
    enter(`sortArray([${join(nums, ", ")}])`, #{ n: N(nums.len()) });

    if nums.len() <= 1 {
        step("base", `[${join(nums, ", ")}] has ${nums.len()} element(s), so it is already sorted.`,
            [ array("segment", nums) ], "return");
        leave(`${join(nums, ",")}`);
        return nums;
    }

    let mid = nums.len() / 2;
    step("mid", `Split at ${mid}. The split itself costs nothing — the values are not looked at.`,
        [ array("segment", nums).ptr("mid", mid).done(before(mid)) ]);

    let left = [];
    for k in 0..mid { left.push(nums[k]); }
    let right = [];
    for k in mid..nums.len() { right.push(nums[k]); }

    step("recL", `Sort the left half, [${join(left, ", ")}].`, [ array("segment", left) ], "call");
    left = msort(left);

    step("recR", `Sort the right half, [${join(right, ", ")}].`, [ array("segment", right) ], "call");
    right = msort(right);

    step("merge", "Both halves are sorted, so merge them.",
        [ array("left half", left), array("right half", right) ], "call");
    let out = merge(left, right);

    leave(`${join(out, ",")}`);
    out
}

fn trace(input) {
    enter("sort", #{ n: N(input.nums.len()) });
    let sorted = msort(input.nums);
    let answer = join(sorted, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    leave(answer);
}
