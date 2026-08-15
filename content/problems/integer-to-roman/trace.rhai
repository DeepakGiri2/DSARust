// Integer to Roman — put the exceptions in the table and greedy just works.
//
// Treated as six special cases, IV/IX/XL/XC/CD/CM turn the conversion into a
// thicket of conditions. Listed as ordinary table entries worth 4, 9, 40, 90,
// 400 and 900, they disappear into the same loop as everything else. The table
// is constructed so that taking the largest value that fits is never a
// mistake — there is no amount where a smaller symbol would do better.

fn trace(input) {
    let num = input.num;
    let original = num;
    let values = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
    let symbols = ["M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I"];

    enter("intToRoman", #{ num: N(num) });

    let table = map_new();
    for i in 0..values.len() { map_set(table, symbols[i], values[i]); }
    step("table", "Thirteen entries, descending. Six of them are the subtractive forms, sitting alongside the plain symbols rather than being special-cased.",
        [ kv("value table (descending)", table) ]);

    let out = "";
    set(#{ out: S(out) });
    step("init", `Build the numeral for ${num}.`,
        [ kv("value table (descending)", table), text("state", [`remaining: ${num}`, `numeral: ""`]) ]);

    for i in 0..values.len() {
        step("loop", `Consider ${symbols[i]} = ${values[i]}.`,
            [ kv("value table (descending)", table).hl([symbols[i]]),
              array("values", values).ptr("i", i),
              text("state", [`remaining: ${num}`, `numeral: "${out}"`]) ]);

        while num >= values[i] {
            step("fits", `${num} ≥ ${values[i]}, so ${symbols[i]} fits. Nothing smaller could ever do better here — that is what makes greedy correct.`,
                [ array("values", values).ptr("i", i).hl([i]),
                  text("state", [`remaining: ${num}`, `numeral: "${out}"`]) ]);
            out += symbols[i];
            set(#{ out: S(out) });
            step("append", `Numeral is now "${out}".`,
                [ text("state", [`remaining: ${num}`, `numeral: "${out}"`]) ]);
            num -= values[i];
            set(#{ num: N(num) });
            log(`take ${symbols[i]}, ${num} left`);
            step("subtract", `${values[i]} accounted for; ${num} still to write.`,
                [ text("state", [`remaining: ${num}`, `numeral: "${out}"`]) ]);
        }
    }

    log(`answer ${out}`, "result");
    result(out);
    step("ret", `${original} = ${out}.`, [ text("answer", [out]) ], "return");
    leave(out);
}
