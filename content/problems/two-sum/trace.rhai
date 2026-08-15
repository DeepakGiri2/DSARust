// Two Sum — one pass with a hash map.
//
// The point of the animation is that the map is doing the remembering: every
// value we walk past is filed away, so the moment its partner shows up the
// answer is already sitting in the map.

const MAP_LABEL = "seen — value → index";

fn trace(input) {
    let nums = input.nums;
    let target = input.target;
    let arr_label = `nums   (target = ${target})`;
    let seen = map_new();

    enter("twoSum", #{ nums: A(nums), target: N(target) });
    set(#{ seen: MAP(seen) });
    step("init",
        "Start with an empty hash map. It will remember every value we have visited, and the index it was at.",
        [ array(arr_label, nums), kv(global::MAP_LABEL, seen) ]);

    for i in 0..nums.len() {
        let x = nums[i];
        set(#{ i: N(i), x: N(x) });
        step("loop", `i = ${i}: look at nums[${i}] = ${x}.`,
            [ array(arr_label, nums).ptr("i", i).done(before(i)), kv(global::MAP_LABEL, seen) ]);

        let need = target - x;
        set(#{ need: N(need) });
        step("need",
            `To reach ${target}, the partner of ${x} has to be ${target} − ${x} = ${need}. That is the only value worth looking for.`,
            [ array(arr_label, nums).ptr("i", i).done(before(i)), kv(global::MAP_LABEL, seen) ]);

        if map_has(seen, need) {
            let j = map_get(seen, need);
            log(`map lookup: seen[${need}] = ${j} → HIT`, "result");
            step("check",
                `Look ${need} up in the map — it is there, filed at index ${j}. One lookup, no scanning.`,
                [ array(arr_label, nums).ptr("i", i).hl([j, i]),
                  kv(global::MAP_LABEL, seen).hl([need]) ]);

            log(`answer: [${j}, ${i}]`, "result");
            result(`[${j}, ${i}]`);
            step("found",
                `Return [${j}, ${i}] — nums[${j}] + nums[${i}] = ${nums[j]} + ${nums[i]} = ${target}. ✓`,
                [ array(arr_label, nums).hl([j, i]), kv(global::MAP_LABEL, seen).hl([need]) ],
                "return");
            leave(`[${j}, ${i}]`);
            return;
        }

        log(`map lookup: seen[${need}] → miss`);
        step("check", `Look ${need} up in the map — not there yet, so ${x} has no partner behind it.`,
            [ array(arr_label, nums).ptr("i", i).done(before(i)), kv(global::MAP_LABEL, seen) ]);

        map_set(seen, x, i);
        set(#{ seen: MAP(seen, [x]) });
        log(`map store: seen[${x}] = ${i}`);
        step("store", `File seen[${x}] = ${i} so a later element can find ${x} in one lookup.`,
            [ array(arr_label, nums).ptr("i", i).done(before(i)),
              kv(global::MAP_LABEL, seen).hl([x]) ]);
    }

    result("[]");
    step("none", "Walked the whole array without ever finding a partner — there is no such pair.",
        [ array(arr_label, nums).done(seq(0, nums.len() - 1)), kv(global::MAP_LABEL, seen) ],
        "return");
    leave("[]");
}
