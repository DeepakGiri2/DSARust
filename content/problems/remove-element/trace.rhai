// Remove Element — a write pointer that lags behind the read pointer.
//
// Nothing is deleted; the survivors are simply packed towards the front. The
// gap between i and k is exactly how many values have been removed so far.

fn trace(input) {
    let nums = input.nums;
    let val = input.val;
    let n = nums.len();
    let label = `nums   (remove every ${val})`;
    let k = 0;

    enter("removeElement", #{ val: N(val) });
    set(#{ k: N(k) });
    step("init", "k is where the next kept value will be written.",
        [ array(label, nums).ptr("k", k) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `i = ${i}: nums[${i}] = ${nums[i]}.`,
            [ array(label, nums).pointers(#{ k: k, i: i }).hl(before(k)).done(before(i)) ]);

        if nums[i] != val {
            step("check", `${nums[i]} is not ${val}, so it survives.`,
                [ array(label, nums).pointers(#{ k: k, i: i }).hl(before(k)).done(before(i)) ]);
            nums[k] = nums[i];
            log(`keep ${nums[k]} at ${k}`);
            step("keep", `Write it to position ${k}.`,
                [ array(label, nums).pointers(#{ k: k, i: i }).hl(span(0, k)) ]);
            k += 1;
            set(#{ k: N(k) });
            step("inc", `k = ${k}: ${k} value(s) kept so far.`,
                [ array(label, nums).pointers(#{ k: k, i: i }).hl(before(k)).done(before(i + 1)) ]);
        } else {
            step("check", `${nums[i]} is the value to remove — skip it. k stays at ${k}, so the next survivor lands here.`,
                [ array(label, nums).pointers(#{ k: k, i: i }).bad([i]).hl(before(k)) ]);
        }
    }

    log(`${k} value(s) kept`, "result");
    result(`${k}`);
    step("ret", `The first ${k} positions hold every value that is not ${val}. What lies beyond does not matter.`,
        [ array(label, nums).hl(before(k)).done(after(k - 1, n)) ], "return");
    leave(`${k}`);
}
