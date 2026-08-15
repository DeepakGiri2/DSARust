// Find the Duplicate Number — the array *is* a linked list.
//
// Read index i as a node pointing at nums[i]. Two indices holding the same
// value means two nodes pointing at one — which is a cycle, and its entry
// point is the duplicate. Phase one finds a meeting point inside the cycle;
// phase two walks one pointer from the start until they meet again, which
// provably happens at the entrance.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let slow = 0;
    let fast = 0;

    enter("findDuplicate", #{ nums: A(nums) });
    set(#{ slow: N(slow), fast: N(fast) });
    step("init", "Treat index i as a node pointing at nums[i]. Both pointers start at index 0.",
        [ array("nums (index -> value)", nums).pointers(#{ slow: slow, fast: fast }) ]);

    let guard = 0;
    loop {
        guard += 1;
        if guard > 100 { break; }
        step("phase1", "Phase one: find any meeting point inside the cycle.",
            [ array("nums (index -> value)", nums).pointers(#{ slow: slow, fast: fast }) ]);

        slow = nums[slow];
        set(#{ slow: N(slow) });
        step("slow1", `slow follows one link, to index ${slow}.`,
            [ array("nums (index -> value)", nums).pointers(#{ slow: slow, fast: fast }) ]);

        fast = nums[nums[fast]];
        set(#{ fast: N(fast) });
        step("fast1", `fast follows two, to index ${fast}.`,
            [ array("nums (index -> value)", nums).pointers(#{ slow: slow, fast: fast }) ]);

        if slow == fast {
            step("meet", `Both are at index ${slow}, so there is definitely a cycle.`,
                [ array("nums (index -> value)", nums).pointers(#{ slow: slow }).hl([slow]) ]);
            step("break", "Stop phase one.",
                [ array("nums (index -> value)", nums).hl([slow]) ]);
            break;
        }
    }

    let slow2 = 0;
    set(#{ slow2: N(slow2) });
    step("reset",
        "Phase two: put a second pointer back at the start. The distance from the start to the entrance equals the distance from the meeting point to the entrance, so walking both one step at a time makes them meet exactly there.",
        [ array("nums (index -> value)", nums).pointers(#{ slow: slow, slow2: slow2 }) ]);

    while slow != slow2 {
        step("phase2", "Walk both, one link at a time.",
            [ array("nums (index -> value)", nums).pointers(#{ slow: slow, slow2: slow2 }) ]);
        slow = nums[slow];
        set(#{ slow: N(slow) });
        step("walkA", `slow -> ${slow}.`,
            [ array("nums (index -> value)", nums).pointers(#{ slow: slow, slow2: slow2 }) ]);
        slow2 = nums[slow2];
        set(#{ slow2: N(slow2) });
        step("walkB", `slow2 -> ${slow2}.`,
            [ array("nums (index -> value)", nums).pointers(#{ slow: slow, slow2: slow2 }) ]);
    }

    log(`duplicate ${slow}`, "result");
    result(`${slow}`);
    step("ret", `They met at ${slow} — the entrance of the cycle, and therefore the duplicated value.`,
        [ array("nums (index -> value)", nums).hl([slow]) ], "return");
    leave(`${slow}`);
}
