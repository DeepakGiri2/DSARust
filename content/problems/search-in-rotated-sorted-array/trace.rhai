// Search in Rotated Sorted Array — one half is always sorted.
//
// A rotated array cut at any point leaves at least one sorted half. Work out
// which one, then ask whether the target lies inside it: if yes, search there;
// if no, the answer can only be in the other half. Either way half the array
// disappears.

fn trace(input) {
    let nums = input.nums;
    let target = input.target;
    let n = nums.len();
    let label = `nums (rotated)   (target = ${target})`;
    let lo = 0;
    let hi = n - 1;

    enter("search", #{ nums: A(nums), target: N(target) });
    set(#{ lo: N(lo), hi: N(hi) });
    step("init", "The whole array is in play.",
        [ array(label, nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi) ]);

    while lo <= hi {
        step("loop", `Live range [${lo}..${hi}].`,
            [ array(label, nums).pointers(#{ lo: lo, hi: hi })
                .window(lo, hi).done(dead(n, lo, hi)) ]);

        let mid = lo + (hi - lo) / 2;
        set(#{ mid: N(mid) });
        step("mid", `mid = ${mid}, nums[${mid}] = ${nums[mid]}.`,
            [ array(label, nums).pointers(#{ lo: lo, mid: mid, hi: hi })
                .window(lo, hi).hl([mid]).done(dead(n, lo, hi)) ]);

        if nums[mid] == target {
            log(`found at ${mid}`, "result");
            step("found", `nums[${mid}] is the target.`,
                [ array(label, nums).hl([mid]) ]);
            result(`${mid}`);
            step("ret", `Return ${mid}.`, [ array(label, nums).hl([mid]) ], "return");
            leave(`${mid}`);
            return;
        }

        if nums[lo] <= nums[mid] {
            step("leftsorted", `nums[${lo}] = ${nums[lo]} <= nums[${mid}] = ${nums[mid]}, so the left half is the sorted one.`,
                [ array(label, nums).pointers(#{ lo: lo, mid: mid, hi: hi })
                    .window(lo, mid).done(dead(n, lo, hi)) ]);

            if nums[lo] <= target && target < nums[mid] {
                step("inleft", `${target} lies between ${nums[lo]} and ${nums[mid]}, so it can only be in that sorted half.`,
                    [ array(label, nums).pointers(#{ lo: lo, mid: mid, hi: hi })
                        .window(lo, mid - 1).bad(span(mid, hi)) ]);
                hi = mid - 1;
                set(#{ hi: N(hi) });
                step("goleft", `hi = ${hi}.`,
                    [ array(label, nums).pointers(#{ lo: lo, hi: hi })
                        .window(lo, hi).done(dead(n, lo, hi)) ]);
            } else {
                step("inleft", `${target} is outside the sorted left half, so it must be on the right.`,
                    [ array(label, nums).pointers(#{ lo: lo, mid: mid, hi: hi })
                        .bad(span(lo, mid)) ]);
                lo = mid + 1;
                set(#{ lo: N(lo) });
                step("goright1", `lo = ${lo}.`,
                    [ array(label, nums).pointers(#{ lo: lo, hi: hi })
                        .window(lo, hi).done(dead(n, lo, hi)) ]);
            }
        } else {
            step("rightsorted", `nums[${lo}] > nums[${mid}], so the rotation is in the left half and the right half is sorted.`,
                [ array(label, nums).pointers(#{ lo: lo, mid: mid, hi: hi })
                    .window(mid, hi).done(dead(n, lo, hi)) ]);

            if nums[mid] < target && target <= nums[hi] {
                step("inright", `${target} lies between ${nums[mid]} and ${nums[hi]}, so search the sorted right half.`,
                    [ array(label, nums).pointers(#{ lo: lo, mid: mid, hi: hi })
                        .window(mid + 1, hi).bad(span(lo, mid)) ]);
                lo = mid + 1;
                set(#{ lo: N(lo) });
                step("goright2", `lo = ${lo}.`,
                    [ array(label, nums).pointers(#{ lo: lo, hi: hi })
                        .window(lo, hi).done(dead(n, lo, hi)) ]);
            } else {
                step("inright", `${target} is outside the sorted right half, so it must be on the left.`,
                    [ array(label, nums).pointers(#{ lo: lo, mid: mid, hi: hi })
                        .bad(span(mid, hi)) ]);
                hi = mid - 1;
                set(#{ hi: N(hi) });
                step("goleft2", `hi = ${hi}.`,
                    [ array(label, nums).pointers(#{ lo: lo, hi: hi })
                        .window(lo, hi).done(dead(n, lo, hi)) ]);
            }
        }
    }

    log("not present", "result");
    result("-1");
    step("none", `The live range is empty, so ${target} is not in the array.`,
        [ array(label, nums).done(seq(0, n - 1)) ], "return");
    leave("-1");
}
