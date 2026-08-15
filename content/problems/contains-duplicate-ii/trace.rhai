// Contains Duplicate II — a set holding only the last k values.
//
// The question is not "is there a duplicate" but "is there a duplicate *close
// together*". Keeping the window at exactly k elements means any hit inside it
// is automatically within distance k — the distance check disappears.

fn trace(input) {
    let nums = input.nums;
    let k = input.k;
    let n = nums.len();
    let win = set_new();
    let label = `nums   (k = ${k})`;

    enter("containsNearbyDuplicate", #{ k: N(k) });
    set(#{ window: SET(set_items(win)) });
    step("init", `A set that will never hold more than ${k} value(s) — the last ${k} we walked past.`,
        [ array(label, nums), kv("window", win) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `i = ${i}: nums[${i}] = ${nums[i]}.`,
            [ array(label, nums).ptr("i", i).window(if i > k { i - k } else { 0 }, i).done(before(i)),
              kv("window", win) ]);

        if i > k {
            let gone = nums[i - k - 1];
            step("evict", `Index ${i - k - 1} is now more than ${k} away, so ${gone} can no longer form a close pair.`,
                [ array(label, nums).ptr("i", i).bad([i - k - 1]).done(before(i)),
                  kv("window", win).hl([gone]) ]);
            set_del(win, gone);
            set(#{ window: SET(set_items(win)) });
            step("remove", `Drop it. The window holds ${set_len(win)} value(s).`,
                [ array(label, nums).ptr("i", i).window(i - k, i).done(before(i - k)),
                  kv("window", win) ]);
        }

        if set_has(win, nums[i]) {
            log(`${nums[i]} is already in the window`, "result");
            result("true");
            step("check", `${nums[i]} is already in the window, so its previous copy is at most ${k} indices back.`,
                [ array(label, nums).ptr("i", i).hl([i]), kv("window", win).hl([nums[i]]) ]);
            step("dup", "Return true.", [ array(label, nums).hl([i]) ], "return");
            leave("true");
            return;
        }
        step("check", `${nums[i]} is not in the window.`,
            [ array(label, nums).ptr("i", i).done(before(i)), kv("window", win) ]);

        set_add(win, nums[i]);
        set(#{ window: SET(set_items(win)) });
        step("store", `Add it: the window is now ${set_len(win)} wide.`,
            [ array(label, nums).ptr("i", i).done(before(i)), kv("window", win).hl([nums[i]]) ]);
    }

    log("no nearby duplicate", "result");
    result("false");
    step("none", `Duplicates may exist, but never within ${k} indices of each other.`,
        [ array(label, nums).done(seq(0, n - 1)), kv("window", win) ], "return");
    leave("false");
}
