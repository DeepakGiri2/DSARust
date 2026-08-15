// 4Sum — two loops, then two pointers, and four places to skip duplicates.
//
// With the array sorted, the innermost pair needs no loop of its own. If the
// four values sum to less than the target, the only way to increase the total
// is to raise the smaller of the pair; if more, lower the larger. Each step
// therefore eliminates a whole column or row of possibilities.
//
// Uniqueness falls out of the same sorting. Equal values sit next to each
// other, so skipping a value identical to the one just used — at each of the
// four positions — produces every quadruplet exactly once, with no set of
// results to consult.

fn trace(input) {
    let nums = input.nums;
    let target = input.target;
    let n = nums.len();

    enter("fourSum", #{ target: N(target) });

    for i in 1..n {
        let j = i;
        while j > 0 && nums[j - 1] > nums[j] {
            swap_at(nums, j - 1, j);
            j -= 1;
        }
    }
    step("sort", "Sorting does two jobs: it makes the two-pointer squeeze valid, and it puts equal values next to each other so duplicates can be skipped.",
        [ array("nums (sorted)", nums) ]);

    let res = [];
    step("init", "The unique quadruplets.", [ text("quadruplets", res) ]);

    for i in 0..n {
        if i + 3 >= n { break; }

        step("loopI", `First value: index ${i}, ${nums[i]}.`,
            [ array("nums (sorted)", nums).ptr("i", i).hl([i]) ]);

        if i > 0 && nums[i] == nums[i - 1] {
            step("dupI", `${nums[i]} was already used as the first value, and would repeat every quadruplet it produced.`,
                [ array("nums (sorted)", nums).ptr("i", i).bad([i]) ]);
            continue;
        }

        for j in (i + 1)..n {
            if j + 2 >= n { break; }

            step("loopJ", `Second value: index ${j}, ${nums[j]}.`,
                [ array("nums (sorted)", nums).pointers(#{ i: i, j: j }).hl([i, j]) ]);

            if j > i + 1 && nums[j] == nums[j - 1] {
                step("dupJ", `${nums[j]} was already used as the second value alongside ${nums[i]}.`,
                    [ array("nums (sorted)", nums).ptr("j", j).bad([j]) ]);
                continue;
            }

            let lo = j + 1;
            let hi = n - 1;
            step("pointers", `Squeeze the remaining pair from ${lo} and ${hi}.`,
                [ array("nums (sorted)", nums).pointers(#{ i: i, j: j, lo: lo, hi: hi })
                    .hl([i, j]).window(lo, hi) ]);

            while lo < hi {
                step("squeeze", `Window [${lo}..${hi}].`,
                    [ array("nums (sorted)", nums).pointers(#{ lo: lo, hi: hi })
                        .hl([i, j]).window(lo, hi) ]);

                let sum = nums[i] + nums[j] + nums[lo] + nums[hi];
                step("sum", `${nums[i]} + ${nums[j]} + ${nums[lo]} + ${nums[hi]} = ${sum}, against a target of ${target}.`,
                    [ array("nums (sorted)", nums).pointers(#{ lo: lo, hi: hi })
                        .hl([i, j, lo, hi]) ]);

                if sum < target {
                    step("tooSmall", `${sum} is short. Everything left of ${hi} is no larger, so the only way to gain is to raise the small end.`,
                        [ array("nums (sorted)", nums).ptr("lo", lo).window(lo, hi) ]);
                    lo += 1;
                } else if sum > target {
                    step("tooBig", `${sum} overshoots, so the large end must come down.`,
                        [ array("nums (sorted)", nums).ptr("hi", hi).window(lo, hi) ]);
                    hi -= 1;
                } else {
                    step("hit", `Exactly ${target}.`,
                        [ array("nums (sorted)", nums).hl([i, j, lo, hi]) ]);
                    res.push(`${nums[i]},${nums[j]},${nums[lo]},${nums[hi]}`);
                    log(`found ${nums[i]},${nums[j]},${nums[lo]},${nums[hi]}`, "result");
                    step("record", `Record it — ${res.len()} so far.`, [ text("quadruplets", res) ]);

                    lo += 1;
                    while lo < hi && nums[lo] == nums[lo - 1] { lo += 1; }
                    step("skipLo", `Move past every copy of that third value: lo = ${lo}.`,
                        [ array("nums (sorted)", nums).ptr("lo", lo).window(lo, hi) ]);

                    hi -= 1;
                    while lo < hi && nums[hi] == nums[hi + 1] { hi -= 1; }
                    step("skipHi", `And past every copy of the fourth: hi = ${hi}.`,
                        [ array("nums (sorted)", nums).ptr("hi", hi).window(lo, hi) ]);
                }
            }
        }
    }

    let answer = if res.len() == 0 { "(none)" } else { join(res, " ") };
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(res.len(), "unique quadruplet")}.`, [ text("quadruplets", res) ], "return");
    leave(answer);
}
