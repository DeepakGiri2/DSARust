// Maximum Average Subarray I — a fixed-size window that slides.
//
// Every window has the same width, so the average is decided by the sum, and
// the sum of the next window is the current one plus the entering value minus
// the leaving one. Two additions instead of k.

fn trace(input) {
    let nums = input.nums;
    let k = input.k;
    let n = nums.len();

    enter("findMaxAverage", #{ k: N(k) });
    let sum = 0;
    set(#{ sum: N(sum) });
    step("initSum", `Build the first window of ${k} value(s) the slow way — once.`,
        [ array("nums", nums) ]);

    for i in 0..k {
        sum += nums[i];
        set(#{ sum: N(sum) });
        step("fill", `Add nums[${i}] = ${nums[i]}.`,
            [ array("nums", nums).ptr("i", i).window(0, i).hl([i]) ]);
    }
    step("add", `The first window [0..${k - 1}] sums to ${sum}.`,
        [ array("nums", nums).window(0, k - 1) ]);

    let best = sum;
    let best_lo = 0;
    set(#{ best: N(best) });
    step("best", `Best sum so far: ${best}.`, [ array("nums", nums).hl(span(0, k - 1)) ]);

    for i in k..n {
        set(#{ i: N(i) });
        step("loop", `Slide the window right so it ends at ${i}.`,
            [ array("nums", nums).ptr("i", i).window(i - k + 1, i).done(before(i - k + 1)) ]);

        let out = nums[i - k];
        sum += nums[i] - out;
        set(#{ sum: N(sum) });
        log(`+${nums[i]} -${out} -> ${sum}`);
        step("slide", `${nums[i]} enters and ${out} leaves: ${sum}. No re-adding of the ${k - 1} values in the middle.`,
            [ array("nums", nums).ptr("i", i).window(i - k + 1, i)
                .hl([i]).bad([i - k]).done(before(i - k)) ]);

        if sum > best {
            step("cmp", `${sum} beats the best sum ${best}.`,
                [ array("nums", nums).window(i - k + 1, i).hl(span(i - k + 1, i)) ]);
            best = sum;
            best_lo = i - k + 1;
            set(#{ best: N(best) });
            step("newBest", `New best window [${best_lo}..${i}] with sum ${best}.`,
                [ array("nums", nums).hl(span(best_lo, i)) ]);
        } else {
            step("cmp", `${sum} does not beat ${best}.`,
                [ array("nums", nums).window(i - k + 1, i).hl(span(best_lo, best_lo + k - 1)) ]);
        }
    }

    let avg = best / k;
    log(`best sum ${best} over ${k} -> ${avg}`, "result");
    result(`${avg}`);
    step("ret", `The best window is [${best_lo}..${best_lo + k - 1}], sum ${best}, average ${best} / ${k}.`,
        [ array("nums", nums).hl(span(best_lo, best_lo + k - 1)) ], "return");
    leave(`${avg}`);
}
