// Jump Game II — breadth-first search on a line, without a queue.
//
// Everything reachable in j jumps forms a contiguous window. While scanning
// that window, `far` collects the furthest any of its members can reach — and
// that is precisely the next window. Reaching the window's end means one more
// jump had to be spent.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let jumps = 0;
    let end = 0;
    let far = 0;

    enter("jump", #{ nums: A(nums) });
    set(#{ jumps: N(jumps), end: N(end), far: N(far) });
    step("init", "end marks the last index reachable with the jumps spent so far; far is the best any of them can do.",
        [ array("nums", nums).pointers(#{ end: end }).window(0, end) ]);

    for i in 0..(n - 1) {
        set(#{ i: N(i) });
        step("loop", `i = ${i}, inside the level reachable in ${jumps} jump(s).`,
            [ array("nums", nums).pointers(#{ i: i, end: end, far: far })
                .window(0, end).done(before(i)) ]);

        if i + nums[i] > far {
            step("reach", `From ${i} you can jump ${nums[i]}, landing as far as ${i + nums[i]} — further than ${far}.`,
                [ array("nums", nums).pointers(#{ i: i, far: far }).hl([i]).window(0, end) ]);
            far = i + nums[i];
            if far > n - 1 { far = n - 1; }
            set(#{ far: N(far) });
            step("setFar", `far = ${far}. This is the reach of the *next* level.`,
                [ array("nums", nums).pointers(#{ i: i, end: end, far: far }).window(0, far) ]);
        } else {
            step("reach", `From ${i} you reach ${i + nums[i]}, no further than ${far}.`,
                [ array("nums", nums).pointers(#{ i: i, far: far }).window(0, end) ]);
        }

        if i == end {
            step("boundary", `Index ${i} is the last one reachable in ${jumps} jump(s) — the level ends here.`,
                [ array("nums", nums).pointers(#{ i: i, end: end }).hl([i]).window(0, end) ]);
            jumps += 1;
            set(#{ jumps: N(jumps) });
            log(`jump ${jumps} taken`);
            step("count", `Spend a jump: ${jumps} so far.`,
                [ array("nums", nums).pointers(#{ i: i, far: far }).window(0, far) ]);
            end = far;
            set(#{ end: N(end) });
            step("extend", `The new level reaches to ${end}.`,
                [ array("nums", nums).pointers(#{ i: i, end: end }).window(0, end) ]);
        } else {
            step("boundary", `Not the end of the level yet (${end} is).`,
                [ array("nums", nums).pointers(#{ i: i, end: end }).window(0, end) ]);
        }
    }

    log(`answer ${jumps}`, "result");
    result(`${jumps}`);
    step("ret", `The last index falls inside the level reached after ${jumps} jump(s).`,
        [ array("nums", nums).hl([n - 1]).done(before(n - 1)) ], "return");
    leave(`${jumps}`);
}
