// Binary Search — the halving is the whole story.
//
// Every step dims the cells that have just been proven impossible, so the
// live range visibly collapses toward the answer instead of the viewer having
// to track lo and hi in their head.

fn trace(input) {
    let nums = input.nums;
    let target = input.target;
    let n = nums.len();
    let label = `nums   (sorted, target = ${target})`;

    let lo = 0;
    let hi = n - 1;

    enter("search", #{ nums: A(nums), target: N(target) });
    set(#{ lo: N(lo), hi: N(hi) });
    step("init", `Nothing is ruled out yet: the answer, if it exists, is somewhere in [${lo}..${hi}].`,
        [ array(label, nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi) ]);

    while lo <= hi {
        let live = hi - lo + 1;
        step("loop", `The live range [${lo}..${hi}] still holds ${live} candidate(s), so keep going.`,
            [ array(label, nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi).done(dead(n, lo, hi)) ]);

        let mid = lo + (hi - lo) / 2;
        set(#{ mid: N(mid) });
        log(`mid = ${lo} + (${hi} - ${lo})/2 = ${mid}, nums[${mid}] = ${nums[mid]}`);
        step("mid", `Probe the middle: mid = ${mid}, nums[${mid}] = ${nums[mid]}.`,
            [ array(label, nums).pointers(#{ lo: lo, mid: mid, hi: hi }).window(lo, hi).done(dead(n, lo, hi)) ]);

        if nums[mid] == target {
            log(`${nums[mid]} == ${target} → found at index ${mid}`, "result");
            step("cmp", `nums[${mid}] = ${nums[mid]} is exactly the target.`,
                [ array(label, nums).pointers(#{ mid: mid }).hl([mid]).done(dead(n, lo, hi)) ]);
            result(`${mid}`);
            step("found", `Return ${mid}.`, [ array(label, nums).hl([mid]) ], "return");
            leave(`${mid}`);
            return;
        }

        step("cmp", `nums[${mid}] = ${nums[mid]} is not ${target}, so index ${mid} is out — and so is half the range.`,
            [ array(label, nums).pointers(#{ lo: lo, mid: mid, hi: hi }).window(lo, hi).done(dead(n, lo, hi)) ]);

        if nums[mid] < target {
            step("less", `${nums[mid]} < ${target}. The array is sorted, so everything at or left of ${mid} is too small.`,
                [ array(label, nums).pointers(#{ lo: lo, mid: mid, hi: hi }).window(lo, hi)
                    .bad(span(lo, mid)).done(dead(n, lo, hi)) ]);
            lo = mid + 1;
            set(#{ lo: N(lo) });
            log(`discard the left half → lo = ${lo}`);
            step("right", `Move lo to ${lo}. ${live} candidates just became ${hi - lo + 1}.`,
                [ array(label, nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi).done(dead(n, lo, hi)) ]);
        } else {
            step("less", `${nums[mid]} > ${target}. Everything at or right of ${mid} is too big.`,
                [ array(label, nums).pointers(#{ lo: lo, mid: mid, hi: hi }).window(lo, hi)
                    .bad(span(mid, hi)).done(dead(n, lo, hi)) ]);
            hi = mid - 1;
            set(#{ hi: N(hi) });
            log(`discard the right half → hi = ${hi}`);
            step("left", `Move hi to ${hi}. ${live} candidates just became ${hi - lo + 1}.`,
                [ array(label, nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi).done(dead(n, lo, hi)) ]);
        }
    }

    log(`lo (${lo}) passed hi (${hi}) — the range is empty`, "result");
    result("-1");
    step("none", `lo has crossed hi, so the live range is empty: ${target} is not in the array. Return -1.`,
        [ array(label, nums).done(seq(0, n - 1)) ], "return");
    leave("-1");
}
