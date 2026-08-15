// Frequency of the Most Frequent Element — how wide a window can we afford to
// level up?
//
// After sorting, raising every value in a window to its largest member costs
// `max * width - sum`. Keep that under k and the window is affordable; its
// width is the frequency achievable.

fn trace(input) {
    let nums = input.nums;
    let k = input.k;
    let n = nums.len();

    for i in 1..n {
        let j = i;
        while j > 0 && nums[j - 1] > nums[j] {
            swap_at(nums, j - 1, j);
            j -= 1;
        }
    }
    let label = `nums (sorted)   (budget k = ${k})`;

    enter("maxFrequency", #{ k: N(k) });
    step("sort", `Sort first: ${brackets(nums)}. Levelling up is only ever cheapest towards a neighbour.`,
        [ array(label, nums) ]);

    let best = 1;
    let left = 0;
    let sum = 0;
    let best_lo = 0;
    set(#{ best: N(best), left: N(left), windowSum: N(sum) });
    step("init", "A single element always has frequency 1, so that is the floor.",
        [ array(label, nums).ptr("left", left) ]);

    for right in 0..n {
        set(#{ right: N(right) });
        step("loop", `right = ${right}: target value ${nums[right]}.`,
            [ array(label, nums).pointers(#{ left: left, right: right })
                .window(left, right).done(before(left)) ]);

        sum += nums[right];
        set(#{ windowSum: N(sum) });
        step("add", `The window now sums to ${sum}.`,
            [ array(label, nums).pointers(#{ left: left, right: right })
                .window(left, right).hl([right]).done(before(left)) ]);

        while nums[right] * (right - left + 1) - sum > k {
            let cost = nums[right] * (right - left + 1) - sum;
            step("shrink",
                `Raising all ${right - left + 1} value(s) to ${nums[right]} would cost ${nums[right]} × ${right - left + 1} − ${sum} = ${cost}, over the budget of ${k}.`,
                [ array(label, nums).pointers(#{ left: left, right: right })
                    .bad(span(left, right)).done(before(left)) ]);
            sum -= nums[left];
            set(#{ windowSum: N(sum) });
            step("sub", `Drop the smallest member (${nums[left]}) — it is the most expensive to raise.`,
                [ array(label, nums).pointers(#{ left: left, right: right }).bad([left]) ]);
            left += 1;
            set(#{ left: N(left) });
            step("advance", `left = ${left}.`,
                [ array(label, nums).pointers(#{ left: left, right: right })
                    .window(left, right).done(before(left)) ]);
        }

        let width = right - left + 1;
        if width > best {
            step("cmp", `${width} identical values are affordable here, beating ${best}.`,
                [ array(label, nums).hl(span(left, right)) ]);
            best = width;
            best_lo = left;
            set(#{ best: N(best) });
            log(`new best frequency ${best}`, "result");
            step("newBest", `Best frequency is now ${best}.`,
                [ array(label, nums).hl(span(best_lo, best_lo + best - 1)) ]);
        } else {
            step("cmp", `${width} does not beat ${best}.`,
                [ array(label, nums).window(left, right).hl(span(best_lo, best_lo + best - 1)) ]);
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `With ${k} increments the most frequent value can be made to appear ${best} time(s).`,
        [ array(label, nums).hl(span(best_lo, best_lo + best - 1)) ], "return");
    leave(`${best}`);
}
