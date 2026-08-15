// Remove Duplicates II — compare against the output, not the input.
//
// The write pointer never overtakes the read pointer, so writing in place is
// safe. The condition looks back two positions in the *output* rather than the
// input, and that is the whole idea: since the array is sorted, the value two
// slots back in the output being equal means two copies have already been
// kept, so this one is the third and is dropped. Looking back into the input
// would count copies that were themselves discarded.

fn validate(input) {
    for i in 1..input.nums.len() {
        if input.nums[i] < input.nums[i - 1] {
            return "nums must be sorted in non-decreasing order";
        }
    }
}

fn kept(k) {
    let out = [];
    for i in 0..k { out.push(i); }
    out
}

fn trace(input) {
    let nums = input.nums;
    let original = [];
    for v in nums { original.push(v); }
    let n = nums.len();

    enter("removeDuplicates", #{ n: N(n) });

    let k = 0;
    set(#{ k: N(k) });
    step("init", "k is both the write position and the length of the answer so far.",
        [ array("nums", nums).ptr("k", k) ]);

    for i in 0..n {
        let v = original[i];
        step("loop", `Read index ${i}: ${v}.`,
            [ array("input (original order)", original).ptr("i", i).done(before(i)),
              array("nums (being rewritten)", nums).ptr("k", k).done(kept(k)) ]);

        if k < 2 || v != nums[k - 2] {
            step("allowed",
                if k < 2 { `Fewer than two values written, so ${v} is allowed whatever it is.` }
                else { `Two slots back in the *output* holds ${nums[k - 2]}. It is not ${v}, so fewer than two copies of ${v} have been kept.` },
                [ array("nums (being rewritten)", nums)
                    .pointers(#{ k: k })
                    .hl(if k >= 2 { [k - 2] } else { [] }).done(kept(k)) ]);

            nums[k] = v;
            step("write", `Write ${v} at position ${k}. The write pointer never passes the read pointer, so this cannot clobber anything unread.`,
                [ array("nums (being rewritten)", nums).ptr("k", k).hl([k]).done(kept(k)) ]);

            k += 1;
            set(#{ k: N(k) });
            log(`keep ${v}; length ${k}`);
            step("bump", `Length is now ${k}.`,
                [ array("nums (being rewritten)", nums).ptr("k", k).done(kept(k)) ]);
        } else {
            step("allowed", `Two slots back in the output already holds ${v}, so two copies are kept and this is a third — drop it.`,
                [ array("input (original order)", original).ptr("i", i).bad([i]),
                  array("nums (being rewritten)", nums).hl([k - 2, k - 1]).done(kept(k)) ]);
        }
    }

    let front = [];
    for i in 0..k { front.push(nums[i]); }
    let answer = `${k} | ${join(front, " ")}`;
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `The first ${k} values are the answer: ${join(front, " ")}. Everything past ${k} is ignored.`,
        [ array("nums (being rewritten)", nums).ptr("k", k).done(kept(k)) ], "return");
    leave(answer);
}
