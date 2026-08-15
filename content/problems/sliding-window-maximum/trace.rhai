// Sliding Window Maximum — a monotonic deque of *indices*.
//
// The deque holds candidates in decreasing order of value. A new value larger
// than the ones behind it makes them permanently useless — they are older and
// smaller, so they can never be the maximum again. The front is therefore
// always the answer for the current window.

fn labels(deque, nums) {
    let out = [];
    for i in deque { out.push(`${nums[i]}@${i}`); }
    out
}

fn trace(input) {
    let nums = input.nums;
    let k = input.k;
    let n = nums.len();
    let label = `nums   (window = ${k})`;
    let res = [];
    let deque = [];
    let left = 0;

    enter("maxSlidingWindow", #{ k: N(k) });
    set(#{ res: A(res) });
    step("res", "Collect one maximum per window.", [ array(label, nums) ]);
    set(#{ deque: A(labels(deque, nums)) });
    step("deque", "The deque holds indices whose values decrease from front to back.",
        [ array(label, nums), deque("deque (value@index)", labels(deque, nums)) ]);
    set(#{ left: N(left) });
    step("left", "left is the first index still inside the window.",
        [ array(label, nums).ptr("left", left),
          deque("deque (value@index)", labels(deque, nums)) ]);

    for right in 0..n {
        set(#{ right: N(right) });
        step("loop", `right = ${right}: nums[${right}] = ${nums[right]}.`,
            [ array(label, nums).pointers(#{ left: left, right: right })
                .window(left, right).done(before(left)),
              deque("deque (value@index)", labels(deque, nums)) ]);

        while deque.len() > 0 && nums[deque[deque.len() - 1]] < nums[right] {
            let dead_idx = deque[deque.len() - 1];
            step("pop", `nums[${dead_idx}] = ${nums[dead_idx]} is smaller than ${nums[right]} and older — it can never be a maximum again.`,
                [ array(label, nums).pointers(#{ left: left, right: right }).bad([dead_idx]),
                  deque("deque (value@index)", labels(deque, nums)) ]);
            deque.truncate(deque.len() - 1);
            set(#{ deque: A(labels(deque, nums)) });
            step("popBack", "Drop it from the back.",
                [ array(label, nums).pointers(#{ left: left, right: right }),
                  deque("deque (value@index)", labels(deque, nums))
                      .popped(`${nums[dead_idx]}@${dead_idx}`) ]);
        }

        deque.push(right);
        set(#{ deque: A(labels(deque, nums)) });
        step("push", `Push index ${right}. The deque stays decreasing.`,
            [ array(label, nums).pointers(#{ left: left, right: right }).hl([right]),
              deque("deque (value@index)", labels(deque, nums)).pushed(true) ]);

        if deque[0] < left {
            let gone = deque[0];
            step("expire", `Index ${gone} at the front has fallen out of the window.`,
                [ array(label, nums).pointers(#{ left: left, right: right }).bad([gone]),
                  deque("deque (value@index)", labels(deque, nums)) ]);
            deque.remove(0);
            set(#{ deque: A(labels(deque, nums)) });
            step("popFront", "Drop it from the front.",
                [ array(label, nums).pointers(#{ left: left, right: right }),
                  deque("deque (value@index)", labels(deque, nums))
                      .popped(`${nums[gone]}@${gone}`) ]);
        }

        if right + 1 >= k {
            let best = nums[deque[0]];
            res.push(best);
            set(#{ res: A(res) });
            log(`window [${left}..${right}] max = ${best}`, "result");
            step("ready", `The window [${left}..${right}] is full, and the front of the deque is its maximum.`,
                [ array(label, nums).window(left, right).hl([deque[0]]),
                  deque("deque (value@index)", labels(deque, nums)) ]);
            step("record", `Record ${best}.`,
                [ array(label, nums).window(left, right).hl([deque[0]]),
                  array("result", res).hl([res.len() - 1]) ]);
            left += 1;
            set(#{ left: N(left) });
            step("advance", `left = ${left}.`,
                [ array(label, nums).pointers(#{ left: left, right: right }).done(before(left)),
                  deque("deque (value@index)", labels(deque, nums)) ]);
        }
    }

    log(`answer ${brackets(res)}`, "result");
    result(brackets(res));
    step("ret", `Every window's maximum, in order: ${brackets(res)} — each index entered and left the deque at most once.`,
        [ array(label, nums), array("result", res).hl(seq(0, res.len() - 1)) ], "return");
    leave(brackets(res));
}
