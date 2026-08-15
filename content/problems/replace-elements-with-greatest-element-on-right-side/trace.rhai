// Replace Elements with Greatest Element on Right Side — walk backwards.
//
// Going right to left, the answer for a cell is already in hand before the
// cell is overwritten. Going forwards would need a fresh scan of the suffix
// for every position.

fn trace(input) {
    let arr = input.arr;
    let n = arr.len();
    let mx = -1;

    enter("replaceElements", #{ arr: A(arr) });
    set(#{ mx: N(mx) });
    step("init", "The last cell has nothing to its right, so its answer is −1 — and that is where the scan starts.",
        [ array("arr", arr) ]);

    let i = n - 1;
    while i >= 0 {
        set(#{ i: N(i) });
        step("loop", `i = ${i}. Everything to the right has already been processed.`,
            [ array("arr", arr).ptr("i", i).done(after(i, n)) ]);

        let cur = arr[i];
        set(#{ cur: N(cur) });
        step("read", `Remember the original value ${cur} before overwriting it — it is needed for the cells further left.`,
            [ array("arr", arr).ptr("i", i).hl([i]).done(after(i, n)) ]);

        arr[i] = mx;
        log(`arr[${i}] = ${mx}`);
        step("replace", `arr[${i}] = ${mx}, the largest value to its right.`,
            [ array("arr", arr).ptr("i", i).hl([i]).done(after(i, n)) ]);

        if cur > mx {
            step("check", `${cur} is larger than the running maximum ${mx}.`,
                [ array("arr", arr).ptr("i", i).done(after(i, n)) ]);
            mx = cur;
            set(#{ mx: N(mx) });
            step("update", `mx = ${mx}.`,
                [ array("arr", arr).ptr("i", i).done(after(i - 1, n)) ]);
        } else {
            step("check", `${cur} does not beat ${mx}, so the running maximum stands.`,
                [ array("arr", arr).ptr("i", i).done(after(i - 1, n)) ]);
        }
        i -= 1;
    }

    let out = brackets(arr);
    log(`answer ${out}`, "result");
    result(out);
    step("ret", `One backwards pass: ${out}.`, [ array("arr", arr).hl(seq(0, n - 1)) ], "return");
    leave(out);
}
