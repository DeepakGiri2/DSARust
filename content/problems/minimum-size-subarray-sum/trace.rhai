// Minimum Size Subarray Sum — grow right, then shrink left.
//
// All values are positive, which is what makes the window monotone: adding on
// the right only ever increases the sum, and removing from the left only ever
// decreases it. So once the window reaches the target there is no reason to
// keep the extra length — shrink until it stops qualifying.

fn trace(input) {
    let target = input.target;
    let nums = input.nums;
    let n = nums.len();
    let label = `nums   (target = ${target})`;

    let left = 0;
    let sum = 0;
    let best = 0;
    let best_lo = 0;
    let best_hi = 0;

    enter("minSubArrayLen", #{ target: N(target) });
    set(#{ left: N(left), sum: N(sum) });
    step("init", "An empty window at the start of the array.", [ array(label, nums).ptr("left", 0) ]);
    set(#{ best: NIL() });
    step("init2", "No qualifying window found yet.", [ array(label, nums).ptr("left", 0) ]);

    for right in 0..n {
        set(#{ right: N(right) });
        step("loop", `right = ${right}.`,
            [ array(label, nums).pointers(#{ left: left, right: right })
                .window(left, right).done(before(left)) ]);

        sum += nums[right];
        set(#{ sum: N(sum) });
        log(`+${nums[right]} -> sum ${sum}`);
        step("add", `Take nums[${right}] = ${nums[right]} into the window: sum ${sum}.`,
            [ array(label, nums).pointers(#{ left: left, right: right })
                .window(left, right).hl([right]).done(before(left)) ]);

        while sum >= target {
            let len = right - left + 1;
            step("shrink", `The window [${left}..${right}] sums to ${sum}, which already reaches ${target} — so its length ${len} is a candidate.`,
                [ array(label, nums).pointers(#{ left: left, right: right })
                    .hl(span(left, right)).done(before(left)) ]);

            if best == 0 || len < best {
                step("cmp", `${len} beats the best length so far${if best == 0 { " (none yet)" } else { `` }}.`,
                    [ array(label, nums).hl(span(left, right)) ]);
                best = len;
                best_lo = left;
                best_hi = right;
                set(#{ best: N(best) });
                log(`new best length ${best} at [${best_lo}..${best_hi}]`, "result");
                step("newMin", `Best is now ${best}.`, [ array(label, nums).hl(span(best_lo, best_hi)) ]);
            } else {
                step("cmp", `${len} does not beat ${best}.`,
                    [ array(label, nums).window(left, right).hl(span(best_lo, best_hi)) ]);
            }

            sum -= nums[left];
            set(#{ sum: N(sum) });
            step("sub", `Drop nums[${left}] = ${nums[left]} from the left: sum ${sum}. Every value is positive, so this can only make the sum smaller.`,
                [ array(label, nums).pointers(#{ left: left, right: right })
                    .bad([left]).done(before(left)) ]);
            left += 1;
            set(#{ left: N(left) });
            step("advance", `left = ${left}.`,
                [ array(label, nums).pointers(#{ left: left, right: right })
                    .window(left, right).done(before(left)) ]);
        }
    }

    if best == 0 {
        step("test", "No window ever reached the target.", [ array(label, nums) ]);
        log("no qualifying subarray", "result");
        result("0");
        step("none", "The whole array sums to less than the target, so return 0.",
            [ array(label, nums).done(seq(0, n - 1)) ], "return");
        leave("0");
        return;
    }

    step("test", `The shortest qualifying window has length ${best}.`,
        [ array(label, nums).hl(span(best_lo, best_hi)) ]);
    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `[${best_lo}..${best_hi}] is the shortest run reaching ${target}: ${best} value(s).`,
        [ array(label, nums).hl(span(best_lo, best_hi)) ], "return");
    leave(`${best}`);
}
