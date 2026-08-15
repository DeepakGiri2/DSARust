fn level_from(nodes, root) {
    if type_of(root) == "()" { return "nil"; }
    let out = [];
    let q = [root];
    let qi = 0;
    while qi < q.len() {
        let id = q[qi];
        qi += 1;
        if type_of(id) == "()" {
            out.push("n");
        } else {
            out.push(`${nodes[id].val}`);
            q.push(nodes[id].left);
            q.push(nodes[id].right);
        }
    }
    while out.len() > 0 && out[out.len() - 1] == "n" { out.truncate(out.len() - 1); }
    join(out, " ")
}

// Sorted Array to BST — the middle element is what makes it balanced.
//
// The whole construction leans on the array being sorted, so say so rather
// than quietly building a tree that is not a BST.
//
// A sorted array *is* an inorder traversal, so whichever element is chosen as
// the root, everything left of it must become the left subtree and everything
// right of it the right subtree. Picking the middle makes those two halves
// equal in size, so the height halves at every level. Balance is a consequence
// of the choice, not something fixed up afterwards.

fn validate(input) {
    for i in 1..input.nums.len() {
        if input.nums[i] < input.nums[i - 1] {
            return `nums must be sorted: ${input.nums[i - 1]} comes before ${input.nums[i]}`;
        }
    }
}

fn build(nums, lo, hi, built) {
    if lo > hi {
        enter(`build(${lo}, ${hi})`);
        step("base", `The range [${lo}..${hi}] is empty, so this child is nil.`,
            [ array("nums", nums), tree("bst", built) ]);
        leave("nil");
        return [built, ()];
    }

    enter(`build(${lo}, ${hi})`, #{ lo: N(lo), hi: N(hi) });
    step("base", `Range [${lo}..${hi}] holds ${hi - lo + 1} value(s), all of which belong in one subtree.`,
        [ array("nums", nums).window(lo, hi).done(dead(nums.len(), lo, hi)),
          tree("bst", built) ]);

    let mid = lo + (hi - lo) / 2;
    step("mid", `The middle is index ${mid}. Splitting there leaves ${mid - lo} value(s) on the left and ${hi - mid} on the right — as even as it can be.`,
        [ array("nums", nums).window(lo, hi).ptr("mid", mid).hl([mid])
            .done(dead(nums.len(), lo, hi)),
          tree("bst", built) ]);

    let me = built.len();
    built.push(#{ id: me, val: nums[mid], left: (), right: () });
    log(`node ${nums[mid]} roots [${lo}..${hi}]`);
    step("make", `Make ${nums[mid]} the root of this range.`,
        [ array("nums", nums).window(lo, hi).hl([mid]).done(dead(nums.len(), lo, hi)),
          tree("bst", built).cur(me) ]);

    step("recL", `Everything below index ${mid} is smaller, so it becomes the left subtree.`,
        [ array("nums", nums).window(lo, hi).done(dead(nums.len(), lo, mid - 1)),
          tree("bst", built).cur(me) ], "call");
    let l = build(nums, lo, mid - 1, built);
    built = l[0];
    built[me].left = l[1];

    step("recR", `Everything above index ${mid} is larger, so it becomes the right subtree.`,
        [ array("nums", nums).window(lo, hi).done(dead(nums.len(), mid + 1, hi)),
          tree("bst", built).cur(me) ], "call");
    let r = build(nums, mid + 1, hi, built);
    built = r[0];
    built[me].right = r[1];

    step("ret", `The subtree rooted at ${nums[mid]} is complete.`,
        [ array("nums", nums), tree("bst", built).cur(me) ], "return");
    leave(`${nums[mid]}`);
    [built, me]
}

fn trace(input) {
    let nums = input.nums;

    enter("sortedArrayToBST", #{ n: N(nums.len()) });
    step("call", "The array is sorted, so it already reads as an inorder traversal — the only freedom is which element to make the root.",
        [ array("nums", nums) ], "call");

    let out = build(nums, 0, nums.len() - 1, []);
    let answer = level_from(out[0], out[1]);
    log(`answer ${answer}`, "result");
    result(answer);
    leave(answer);
}
