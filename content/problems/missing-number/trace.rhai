// Missing Number — XOR pairs every value with its index.
//
// Every number that is present shows up twice: once as a value and once as an
// index somewhere. XOR cancels those pairs to zero. The missing number only
// ever appears as an index, so it is the one thing left standing.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let width = 6;
    let acc = n;

    enter("missingNumber", #{ nums: A(nums) });
    set(#{ acc: N(acc) });
    step("init", `Seed with ${n} — the one index that has no slot in the array.`,
        [ array("nums", nums), bits("accumulator", [["acc", acc, width]]) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `i = ${i}, nums[${i}] = ${nums[i]}.`,
            [ array("nums", nums).ptr("i", i).hl([i]).done(before(i)),
              bits("accumulator", [["acc", acc, width]]) ]);

        acc = acc ^ i;
        set(#{ acc: N(acc) });
        step("xorIdx", `XOR the index ${i}: acc = ${acc}.`,
            [ array("nums", nums).ptr("i", i).done(before(i)),
              bits("accumulator", [["acc", acc, width], [`index ${i}`, i, width]]) ]);

        acc = acc ^ nums[i];
        set(#{ acc: N(acc) });
        log(`^${i} ^${nums[i]} -> ${acc}`);
        step("xorVal", `XOR the value ${nums[i]}: acc = ${acc}.`,
            [ array("nums", nums).ptr("i", i).done(before(i + 1)),
              bits("accumulator", [["acc", acc, width], [`value ${nums[i]}`, nums[i], width]]) ]);
    }

    log(`answer ${acc}`, "result");
    result(`${acc}`);
    step("ret", `Every present number cancelled with its own index. ${acc} is the one that never did.`,
        [ array("nums", nums).done(seq(0, n - 1)),
          bits("accumulator", [["acc", acc, width]]) ], "return");
    leave(`${acc}`);
}
