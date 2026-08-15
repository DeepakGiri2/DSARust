// Baseball Game — every rule refers to the most recent entries, so: a stack.
//
// "+" reads the last two, "D" reads the last one, "C" removes the last one.
// Nothing ever reaches into the middle of the record, which is exactly the
// access pattern a stack gives you for free — and it is why "C" costs nothing
// to undo, rather than forcing the total to be recomputed.

fn trace(input) {
    let ops = words(input.operations);
    let record = [];

    enter("calPoints", #{});
    set(#{ record: A(record) });
    step("init", "The record: only its top few entries are ever consulted.",
        [ array("operations", ops), stack("record", record) ]);

    for i in 0..ops.len() {
        let op = ops[i];
        step("loop", `Operation ${i}: "${op}".`,
            [ array("operations", ops).ptr("i", i).done(before(i)),
              stack("record", record) ]);

        if op == "C" {
            let gone = record[record.len() - 1];
            step("isC", `"C" cancels the previous score, ${gone}.`,
                [ array("operations", ops).ptr("i", i).hl([i]), stack("record", record) ]);
            record.truncate(record.len() - 1);
            set(#{ record: A(record) });
            log(`cancel ${gone}`);
            step("popC", `Drop it. Nothing else has to change, because no later entry was built from it yet.`,
                [ array("operations", ops).ptr("i", i), stack("record", record).popped(gone) ]);
        } else if op == "D" {
            let last = record[record.len() - 1];
            step("isD", `"D" doubles the previous score, ${last}.`,
                [ array("operations", ops).ptr("i", i).hl([i]), stack("record", record) ]);
            record.push(2 * last);
            set(#{ record: A(record) });
            log(`double ${last} -> ${2 * last}`);
            step("pushD", `Record ${2 * last}.`,
                [ array("operations", ops).ptr("i", i), stack("record", record).pushed(true) ]);
        } else if op == "+" {
            let a = record[record.len() - 1];
            let b = record[record.len() - 2];
            step("isPlus", `"+" sums the previous two scores, ${b} and ${a}.`,
                [ array("operations", ops).ptr("i", i).hl([i]), stack("record", record) ]);
            record.push(a + b);
            set(#{ record: A(record) });
            log(`${b} + ${a} = ${a + b}`);
            step("pushPlus", `Record ${a + b}.`,
                [ array("operations", ops).ptr("i", i), stack("record", record).pushed(true) ]);
        } else {
            step("isNum", `"${op}" is a plain score.`,
                [ array("operations", ops).ptr("i", i).hl([i]), stack("record", record) ]);
            record.push(num(op));
            set(#{ record: A(record) });
            step("pushNum", `Record ${op}.`,
                [ array("operations", ops).ptr("i", i), stack("record", record).pushed(true) ]);
        }
    }

    let total = 0;
    set(#{ total: N(total) });
    step("sum", "Everything still on the record counts.",
        [ stack("record", record), array("surviving scores", record) ]);

    for k in 0..record.len() {
        total += record[k];
        set(#{ total: N(total) });
        step("add", `+ ${record[k]} = ${total}.`,
            [ array("surviving scores", record).ptr("k", k).done(before(k)) ]);
    }

    log(`answer ${total}`, "result");
    result(`${total}`);
    step("ret", `The surviving record adds up to ${total}.`,
        [ array("surviving scores", record).done(seq(0, record.len() - 1)) ], "return");
    leave(`${total}`);
}
