// House Robber II — the circle costs one extra run, not a new algorithm.
//
// First and last are adjacent, but that only bites if *both* are robbed. So
// forbid the last house, solve the straight line; forbid the first, solve it
// again; take the better. Every valid circular plan appears in one of the two.

fn rob_line(nums, lo, hi, label, full, marks) {
    enter(`robLine(${lo}..${hi - 1})`);
    let prev = 0;
    let cur = 0;
    set(#{ prev: N(prev), cur: N(cur) });
    step("lineInit", `${label}`,
        [ bars("houses", full).window(lo, hi - 1).done(marks) ]);

    for i in lo..hi {
        step("lineLoop", `House ${i} holds ${full[i]}.`,
            [ bars("houses", full).ptr("i", i).window(lo, hi - 1).done(marks) ]);
        let take = prev + full[i];
        let skip = cur;
        let next = if take > skip { take } else { skip };
        log(`house ${i}: rob=${take} skip=${skip} -> ${next}`);
        step("lineStep",
            `Rob it for ${prev} + ${full[i]} = ${take}, or skip it and keep ${skip}. Best: ${next}.`,
            [ bars("houses", full).ptr("i", i).hl(if take > skip { [i] } else { [] })
                .window(lo, hi - 1).done(marks) ]);
        prev = cur;
        cur = next;
        set(#{ prev: N(prev), cur: N(cur) });
    }

    step("lineRet", `This line yields ${cur}.`,
        [ bars("houses", full).window(lo, hi - 1).done(marks) ], "return");
    leave(`${cur}`);
    cur
}

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("rob", #{ nums: A(nums) });

    if n == 1 {
        step("single", "One house, and nothing is adjacent to itself.", [ bars("houses", nums) ]);
        result(`${nums[0]}`);
        step("retSingle", `Rob it: ${nums[0]}.`, [ bars("houses", nums).hl([0]) ], "return");
        leave(`${nums[0]}`);
        return;
    }
    step("single", `${n} houses in a circle, so house 0 and house ${n - 1} are neighbours.`,
        [ bars("houses", nums).hl([0, n - 1]) ]);

    step("dropLast", `Case 1: forbid the last house. What is left is a straight line, 0..${n - 2}.`,
        [ bars("houses", nums).bad([n - 1]).window(0, n - 2) ], "call");
    let a = rob_line(nums, 0, n - 1, "Straight line, last house excluded.", nums, [n - 1]);

    step("dropFirst", `Case 2: forbid the first house instead. Line 1..${n - 1}.`,
        [ bars("houses", nums).bad([0]).window(1, n - 1) ], "call");
    let b = rob_line(nums, 1, n, "Straight line, first house excluded.", nums, [0]);

    let answer = if a > b { a } else { b };
    log(`max(${a}, ${b}) = ${answer}`, "result");
    result(`${answer}`);
    step("ret",
        `Excluding the last gives ${a}, excluding the first gives ${b}. Every legal circular plan appears in one of those two, so the answer is ${answer}.`,
        [ bars("houses", nums) ], "return");
    leave(`${answer}`);
}
