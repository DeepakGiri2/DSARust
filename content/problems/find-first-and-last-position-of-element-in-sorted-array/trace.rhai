// Find First and Last Position — one boundary search, called twice.
//
// The temptation is to binary-search for the target and then walk outwards to
// the ends of the run. That is O(log n + run), and the run can be the whole
// array. Searching for *boundaries* instead keeps it logarithmic: the first
// index ≥ target, and the first index ≥ target+1. Note that neither loop stops
// early on an exact hit — stopping early is what breaks boundary searches.

const A = "nums";

fn lower_bound(nums, target) {
    enter(`lowerBound(${target})`, #{ target: N(target) });
    let n = nums.len();
    let l = 0;
    let r = n;
    step("init", `Search the half-open range [0, ${n}). r = ${n} — one past the end, so "not found" has somewhere to land.`,
        [ array(global::A, nums).ptr("l", l).ptr("r", r) ]);

    while l < r {
        step("loop", `l = ${l}, r = ${r}: ${r - l} candidate(s) left.`,
            [ array(global::A, nums).ptr("l", l).ptr("r", r).done(dead(n, l, r - 1)) ]);
        let m = (l + r) / 2;
        step("mid", `m = ${m}, nums[${m}] = ${nums[m]}.`,
            [ array(global::A, nums).ptr("l", l).ptr("r", r).ptr("m", m).hl([m]).done(dead(n, l, r - 1)) ]);

        if nums[m] < target {
            step("cmp", `${nums[m]} < ${target} — index ${m} is too far left to be the boundary.`,
                [ array(global::A, nums).ptr("m", m).bad([m]).done(dead(n, l, r - 1)) ]);
            l = m + 1;
            step("right", `l = ${l}.`,
                [ array(global::A, nums).ptr("l", l).ptr("r", r).done(dead(n, l, r - 1)) ]);
        } else {
            step("cmp", `${nums[m]} ≥ ${target} — index ${m} might itself be the boundary, so it stays in range.`,
                [ array(global::A, nums).ptr("m", m).hl([m]).done(dead(n, l, r - 1)) ]);
            r = m;
            step("left", `r = ${r}. Not m − 1: discarding m could discard the answer.`,
                [ array(global::A, nums).ptr("l", l).ptr("r", r).done(dead(n, l, r - 1)) ]);
        }
    }

    step("bound", `l = r = ${l}: the first index whose value is ≥ ${target}.`,
        [ array(global::A, nums).ptr("bound", l) ], "return");
    leave(`${l}`);
    l
}

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let target = input.target;

    enter("searchRange", #{ nums: A(nums), target: N(target) });

    step("first", `First, the lower bound of ${target} — the leftmost place it could sit.`,
        [ array(global::A, nums) ], "call");
    let lo = lower_bound(nums, target);
    set(#{ lo: N(lo) });

    if lo == n || nums[lo] != target {
        log(`${target} is absent`, "result");
        step("absent", `${if lo == n { "The bound ran off the end" } else { `nums[${lo}] = ${nums[lo]}, not ${target}` }} — the target is not in the array at all.`,
            [ array(global::A, nums).bad(seq(0, n - 1)) ]);
        result("-1 -1");
        step("none", "Return -1 -1.", [ array(global::A, nums) ], "return");
        leave("-1 -1");
        return;
    }

    step("last", `${target} starts at index ${lo}. Now the lower bound of ${target + 1}: the first index past the run.`,
        [ array(global::A, nums).ptr("lo", lo).hl([lo]) ], "call");
    let after = lower_bound(nums, target + 1);
    let hi = after - 1;
    set(#{ hi: N(hi) });

    let out = `${lo} ${hi}`;
    log(out, "result");
    result(out);
    step("ret", `The run of ${target} spans indices ${lo} … ${hi} — found in two logarithmic searches, however long the run is.`,
        [ array(global::A, nums).hl(span(lo, hi)).ptr("lo", lo).ptr("hi", hi) ], "return");
    leave(out);
}
