// Find Minimum in Rotated Sorted Array — compare against the *right* end.
//
// Comparing mid with the right-hand element is what makes this work: if
// nums[mid] > nums[hi] the rotation point must be to the right of mid, and
// otherwise mid itself could still be the minimum — so hi moves to mid, never
// past it.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let lo = 0;
    let hi = n - 1;

    enter("findMin", #{ nums: A(nums) });
    set(#{ lo: N(lo), hi: N(hi) });
    step("init", "The array is sorted but rotated, so the minimum is wherever the wrap happens.",
        [ array("nums (rotated)", nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi) ]);

    while lo < hi {
        step("loop", `The rotation point is somewhere in [${lo}..${hi}].`,
            [ array("nums (rotated)", nums).pointers(#{ lo: lo, hi: hi })
                .window(lo, hi).done(dead(n, lo, hi)) ]);

        let mid = lo + (hi - lo) / 2;
        set(#{ mid: N(mid) });
        step("mid", `mid = ${mid}, nums[${mid}] = ${nums[mid]}. Compare it with the right end, nums[${hi}] = ${nums[hi]}.`,
            [ array("nums (rotated)", nums).pointers(#{ lo: lo, mid: mid, hi: hi })
                .window(lo, hi).hl([mid, hi]).done(dead(n, lo, hi)) ]);

        if nums[mid] > nums[hi] {
            step("cmp", `${nums[mid]} > ${nums[hi]}: the sequence drops somewhere after mid, so the minimum is strictly to the right.`,
                [ array("nums (rotated)", nums).pointers(#{ lo: lo, mid: mid, hi: hi })
                    .bad(span(lo, mid)).window(lo, hi) ]);
            lo = mid + 1;
            set(#{ lo: N(lo) });
            step("right", `lo = ${lo}.`,
                [ array("nums (rotated)", nums).pointers(#{ lo: lo, hi: hi })
                    .window(lo, hi).done(dead(n, lo, hi)) ]);
        } else {
            step("cmp", `${nums[mid]} <= ${nums[hi]}: from mid to hi is already in order, so the minimum is at mid or left of it — mid stays a candidate.`,
                [ array("nums (rotated)", nums).pointers(#{ lo: lo, mid: mid, hi: hi })
                    .bad(span(mid + 1, hi)).window(lo, hi) ]);
            hi = mid;
            set(#{ hi: N(hi) });
            step("left", `hi = ${hi}.`,
                [ array("nums (rotated)", nums).pointers(#{ lo: lo, hi: hi })
                    .window(lo, hi).done(dead(n, lo, hi)) ]);
        }
    }

    log(`minimum ${nums[lo]} at index ${lo}`, "result");
    result(`${nums[lo]}`);
    step("done", `lo and hi have met at index ${lo}: the minimum is ${nums[lo]}.`,
        [ array("nums (rotated)", nums).hl([lo]) ], "return");
    leave(`${nums[lo]}`);
}
