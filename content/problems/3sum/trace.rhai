// 3Sum — sort, then a two-pointer sweep per anchor.
//
// Sorting buys two things: the inner search becomes the sorted-array two-sum,
// and duplicates end up adjacent so they can be skipped in O(1). Without the
// sort neither trick is available and the problem is O(n^3).

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    for i in 1..n {
        let j = i;
        while j > 0 && nums[j - 1] > nums[j] {
            swap_at(nums, j - 1, j);
            j -= 1;
        }
    }

    enter("threeSum", #{ nums: A(nums) });
    step("sort", `Sort first: ${brackets(nums)}. Now equal values are neighbours and the array is scannable from both ends.`,
        [ array("nums (sorted)", nums) ]);

    let found = [];
    set(#{ found: A(found) });
    step("init", "Collect the triplets here.", [ array("nums (sorted)", nums) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("iloop", `Anchor on nums[${i}] = ${nums[i]}; look for two more values summing to ${-nums[i]}.`,
            [ array("nums (sorted)", nums).ptr("i", i).hl([i]).done(before(i)) ]);

        if nums[i] > 0 {
            step("neg", `${nums[i]} is positive and the rest of the array is at least as large, so no triplet from here can reach 0.`,
                [ array("nums (sorted)", nums).ptr("i", i).bad(seq(i, n - 1)) ]);
            break;
        }

        if i > 0 && nums[i] == nums[i - 1] {
            step("skipI", `nums[${i}] repeats the previous anchor, which already produced every triplet starting with ${nums[i]}.`,
                [ array("nums (sorted)", nums).ptr("i", i).bad([i]).done(before(i)) ]);
            step("contI", "Skip it to avoid duplicate answers.",
                [ array("nums (sorted)", nums).ptr("i", i).done(before(i + 1)) ]);
            continue;
        }
        step("skipI", `nums[${i}] is a new anchor value.`,
            [ array("nums (sorted)", nums).ptr("i", i).hl([i]).done(before(i)) ]);

        let l = i + 1;
        let r = n - 1;
        set(#{ l: N(l), r: N(r) });
        step("lr", `Two pointers over the remainder: [${l}..${r}].`,
            [ array("nums (sorted)", nums).pointers(#{ i: i, l: l, r: r }).window(l, r).done(before(i + 1)) ]);

        while l < r {
            step("wloop", `Window [${l}..${r}].`,
                [ array("nums (sorted)", nums).pointers(#{ i: i, l: l, r: r })
                    .window(l, r).done(before(i + 1)) ]);

            let sum = nums[i] + nums[l] + nums[r];
            set(#{ sum: N(sum) });
            step("sum", `${nums[i]} + ${nums[l]} + ${nums[r]} = ${sum}.`,
                [ array("nums (sorted)", nums).pointers(#{ i: i, l: l, r: r })
                    .hl([i, l, r]).done(before(i + 1)) ]);

            if sum == 0 {
                let triplet = `[${nums[i]},${nums[l]},${nums[r]}]`;
                found.push(triplet);
                set(#{ found: A(found) });
                log(`found ${triplet}`, "result");
                step("zero", `${sum} is zero — that is a triplet.`,
                    [ array("nums (sorted)", nums).pointers(#{ i: i, l: l, r: r }).hl([i, l, r]) ]);
                step("add", `Record ${triplet}.`,
                    [ array("nums (sorted)", nums).pointers(#{ i: i, l: l, r: r }).hl([i, l, r]),
                      text("triplets", found) ]);

                l += 1;
                r -= 1;
                set(#{ l: N(l), r: N(r) });
                step("advance", "Move both pointers inwards — moving only one could not give zero again.",
                    [ array("nums (sorted)", nums).pointers(#{ i: i, l: l, r: r }).window(l, r) ]);

                while l < r && nums[l] == nums[l - 1] {
                    step("dedup", `nums[${l}] repeats the value just used, so it would produce the same triplet.`,
                        [ array("nums (sorted)", nums).pointers(#{ i: i, l: l, r: r }).bad([l]) ]);
                    l += 1;
                    set(#{ l: N(l) });
                }
            } else if sum < 0 {
                step("moveL", `${sum} < 0, so the sum needs to grow — move l right, where the values are larger.`,
                    [ array("nums (sorted)", nums).pointers(#{ i: i, l: l, r: r })
                        .bad([l]).window(l, r) ]);
                l += 1;
                set(#{ l: N(l) });
            } else {
                step("moveR", `${sum} > 0, so the sum needs to shrink — move r left, where the values are smaller.`,
                    [ array("nums (sorted)", nums).pointers(#{ i: i, l: l, r: r })
                        .bad([r]).window(l, r) ]);
                r -= 1;
                set(#{ r: N(r) });
            }
        }
    }

    let answer = `[${join(found, ", ")}]`;
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(found.len(), "triplet")} sum to zero: ${answer}.`,
        [ array("nums (sorted)", nums), text("triplets", found) ], "return");
    leave(answer);
}
