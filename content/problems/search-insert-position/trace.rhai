// Search Insert Position — one invariant answers both questions at once.
//
// `hi` starts *past* the end and only ever moves to `mid`, never `mid - 1`.
// That keeps the range half-open, and with it a single fact: everything below
// `lo` is strictly smaller than the target, everything from `hi` upwards is
// not. When the two meet there is exactly one index that can satisfy both, and
// it is both "where the target is" and "where it belongs" — so there is no
// not-found branch to write.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let target = input.target;

    enter("searchInsert", #{ target: N(target) });

    let lo = 0;
    let hi = n;
    set(#{ lo: N(lo), hi: N(hi) });
    step("bounds", `Search the half-open range [0..${n}). ${n} is deliberately one past the end — that is the answer when the target is larger than everything.`,
        [ array("nums", nums).pointers(#{ lo: lo, hi: hi }) ]);

    while lo < hi {
        step("loop", `Live range [${lo}..${hi}).`,
            [ array("nums", nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi - 1)
                .done(dead(n, lo, hi - 1)),
              text("invariant", ["everything below lo is < target",
                                 "everything from hi up is >= target"]) ]);

        let mid = lo + (hi - lo) / 2;
        set(#{ mid: N(mid) });
        step("mid", `mid = ${mid}, holding ${nums[mid]}.`,
            [ array("nums", nums).pointers(#{ lo: lo, mid: mid, hi: hi }).hl([mid])
                .window(lo, hi - 1).done(dead(n, lo, hi - 1)) ]);

        if nums[mid] < target {
            step("cmp", `${nums[mid]} < ${target}, so index ${mid} cannot be the answer and neither can anything below it.`,
                [ array("nums", nums).pointers(#{ lo: lo, mid: mid, hi: hi }).bad([mid])
                    .window(lo, hi - 1) ]);
            lo = mid + 1;
            set(#{ lo: N(lo) });
            step("right", `Move lo past it: lo = ${lo}. The invariant still holds.`,
                [ array("nums", nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi - 1)
                    .done(dead(n, lo, hi - 1)) ]);
        } else {
            step("notLess", `${nums[mid]} >= ${target}, so ${mid} is a candidate — it might be the answer itself.`,
                [ array("nums", nums).pointers(#{ lo: lo, mid: mid, hi: hi }).hl([mid])
                    .window(lo, hi - 1) ]);
            hi = mid;
            set(#{ hi: N(hi) });
            step("left", `Move hi *to* mid, not past it — discarding it could throw away the answer.`,
                [ array("nums", nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi - 1)
                    .done(dead(n, lo, hi - 1)) ]);
        }
    }

    log(`answer ${lo}`, "result");
    result(`${lo}`);
    step("ret",
        if lo < n && nums[lo] == target {
            `The range collapsed at ${lo}, where ${target} already sits.`
        } else {
            `The range collapsed at ${lo}. ${target} is not present, and ${lo} is where it would be inserted.`
        },
        [ array("nums", nums).ptr("lo", lo).hl(if lo < n { [lo] } else { [] }) ], "return");
    leave(`${lo}`);
}
