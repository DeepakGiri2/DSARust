// Merge Sorted Array — fill from the back.
//
// Merging forwards would overwrite values in nums1 that have not been read
// yet. Writing from the *end* is safe because the tail of nums1 is exactly the
// empty space reserved for the merge, and it always holds the largest value
// left, so nothing is ever clobbered.

fn trace(input) {
    let a = input.nums1vals;
    let b = input.nums2;
    let m = a.len();
    let n = b.len();

    // nums1 is m real values followed by n empty slots.
    let nums1 = [];
    for v in a { nums1.push(v); }
    for slot in 0..n { nums1.push("_"); }

    let i = m - 1;
    let j = n - 1;
    let k = m + n - 1;

    enter("merge", #{ m: N(m), n: N(n) });
    set(#{ i: N(i), j: N(j), k: N(k) });
    step("init",
        "i and j read the last real value of each array; k writes into the last free slot. All three move backwards.",
        [ array("nums1 (with room for nums2)", nums1).pointers(#{ i: i, k: k }),
          array("nums2", b).ptr("j", j) ]);

    while j >= 0 {
        step("loop", `nums2 still has ${j + 1} value(s) to place.`,
            [ array("nums1 (with room for nums2)", nums1).pointers(#{ i: i, k: k }).done(after(k, m + n)),
              array("nums2", b).ptr("j", j) ]);

        if i >= 0 && nums1[i] > b[j] {
            step("cmp", `nums1[${i}] = ${nums1[i]} is larger than nums2[${j}] = ${b[j]}, so it belongs further right.`,
                [ array("nums1 (with room for nums2)", nums1).pointers(#{ i: i, k: k }).hl([i]),
                  array("nums2", b).ptr("j", j) ]);
            nums1[k] = nums1[i];
            log(`nums1[${k}] = ${nums1[k]} (from nums1)`);
            step("take1", `Copy it into slot ${k}.`,
                [ array("nums1 (with room for nums2)", nums1).pointers(#{ i: i, k: k }).hl([k]),
                  array("nums2", b).ptr("j", j) ]);
            i -= 1;
            set(#{ i: N(i) });
            step("decI", `i = ${i}.`,
                [ array("nums1 (with room for nums2)", nums1).pointers(#{ i: i, k: k }),
                  array("nums2", b).ptr("j", j) ]);
        } else {
            step("cmp",
                if i < 0 {
                    `nums1 is exhausted, so everything left in nums2 goes straight in.`.to_string()
                } else {
                    `nums2[${j}] = ${b[j]} is at least nums1[${i}] = ${nums1[i]}, so it takes the slot.`
                },
                [ array("nums1 (with room for nums2)", nums1).pointers(#{ i: i, k: k }),
                  array("nums2", b).ptr("j", j).hl([j]) ]);
            nums1[k] = b[j];
            log(`nums1[${k}] = ${nums1[k]} (from nums2)`);
            step("take2", `Copy ${b[j]} into slot ${k}.`,
                [ array("nums1 (with room for nums2)", nums1).pointers(#{ i: i, k: k }).hl([k]),
                  array("nums2", b).ptr("j", j) ]);
            j -= 1;
            set(#{ j: N(j) });
            step("decJ", `j = ${j}.`,
                [ array("nums1 (with room for nums2)", nums1).pointers(#{ i: i, k: k }),
                  array("nums2", b).ptr("j", j) ]);
        }

        k -= 1;
        set(#{ k: N(k) });
        step("decK", `The write cursor moves left: k = ${k}. It is never ahead of i, so nothing unread is overwritten.`,
            [ array("nums1 (with room for nums2)", nums1).pointers(#{ i: i, k: k }).done(after(k, m + n)),
              array("nums2", b).ptr("j", j) ]);
    }

    let out = join(nums1, " ");
    log(`merged: ${out}`, "result");
    result(out);
    step("decK", `nums2 is empty, so whatever remains in nums1 is already in place: ${out}.`,
        [ array("nums1 (with room for nums2)", nums1).hl(seq(0, m + n - 1)) ], "return");
    leave(out);
}
