// Single Number — XOR cancels pairs.
//
// The bits view is the explanation: watch each duplicate switch the same bit
// on and then back off, leaving only the bits of the lonely value standing.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let acc = 0;
    let width = 8;

    enter("singleNumber", #{ nums: A(nums) });
    set(#{ result: N(acc) });
    step("init",
        "Start the accumulator at 0. XOR has two properties that make this work: x ^ x = 0, and x ^ 0 = x.",
        [ array("nums", nums),
          bits("accumulator", [["result", acc, width]]) ]);

    for i in 0..n {
        let x = nums[i];
        set(#{ i: N(i), x: N(x) });
        step("loop", `i = ${i}: fold nums[${i}] = ${x} into the accumulator.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              bits("accumulator", [["result", acc, width], [`nums[${i}]`, x, width]]) ]);

        let before_acc = acc;
        acc = acc ^ x;
        // Bits that actually flipped are the ones the two values disagree on.
        let flipped = [];
        for b in 0..width {
            if ((before_acc >> b) & 1) != ((acc >> b) & 1) { flipped.push(b); }
        }
        set(#{ result: N(acc) });
        log(`${before_acc} ^ ${x} = ${acc}`);
        step("xor",
            `${before_acc} ^ ${x} = ${acc}. Every bit the two numbers share cancels to 0; the rest survive.`,
            [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)),
              bits("accumulator", [["result", acc, width, flipped], [`nums[${i}]`, x, width, flipped]]) ]);
    }

    log(`every duplicate cancelled → ${acc}`, "result");
    result(`${acc}`);
    step("done",
        `Each value that appeared twice XOR'd itself back to zero, so what is left — ${acc} — is the number that appeared once.`,
        [ array("nums", nums).done(seq(0, n - 1)),
          bits("accumulator", [["result", acc, width]]) ],
        "return");
    leave(`${acc}`);
}
