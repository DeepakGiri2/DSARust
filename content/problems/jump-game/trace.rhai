// Jump Game — one number is enough.
//
// You never need to know *which* jumps to take, only how far the furthest
// reachable index is. The window shows exactly that: everything inside it is
// reachable, and the scan fails the moment the cursor steps outside.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let reach = 0;

    enter("canJump", #{ nums: A(nums) });
    set(#{ reach: N(reach) });
    step("init", "Standing at index 0, the furthest index reachable so far is 0.",
        [ array("nums", nums).ptr("reach", 0).window(0, 0) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `i = ${i}: can we even get here? Reach is ${reach}.`,
            [ array("nums", nums).pointers(#{ i: i, reach: reach })
                .window(0, reach).done(before(i)) ]);

        if i > reach {
            log(`index ${i} is past reach ${reach}`, "result");
            result("false");
            step("check", `Index ${i} is beyond the furthest reachable index ${reach} — there is a gap that cannot be crossed.`,
                [ array("nums", nums).pointers(#{ i: i, reach: reach })
                    .window(0, reach).bad([i]) ]);
            step("bad", "Return false.",
                [ array("nums", nums).bad([i]).done(span(0, reach)) ], "return");
            leave("false");
            return;
        }

        step("check", `Index ${i} is within reach.`,
            [ array("nums", nums).pointers(#{ i: i, reach: reach })
                .window(0, reach).done(before(i)) ]);

        if i + nums[i] > reach {
            step("cmp", `Jumping ${nums[i]} from here lands on ${i + nums[i]}, further than ${reach}.`,
                [ array("nums", nums).pointers(#{ i: i, reach: reach })
                    .window(0, reach).hl([i]).done(before(i)) ]);
            reach = i + nums[i];
            if reach > n - 1 { reach = n - 1; }
            set(#{ reach: N(reach) });
            log(`reach = ${reach}`);
            step("update", `Extend the reachable range to ${reach}.`,
                [ array("nums", nums).pointers(#{ i: i, reach: reach })
                    .window(0, reach).done(before(i)) ]);
        } else {
            step("cmp", `Jumping from here reaches ${i + nums[i]}, no further than ${reach}.`,
                [ array("nums", nums).pointers(#{ i: i, reach: reach })
                    .window(0, reach).done(before(i)) ]);
        }
    }

    log("reached the last index", "result");
    result("true");
    step("ret", `Every index was reachable as the scan passed it, so the last index is reachable too.`,
        [ array("nums", nums).hl([n - 1]).done(before(n - 1)) ], "return");
    leave("true");
}
