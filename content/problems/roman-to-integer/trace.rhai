// Roman to Integer — the subtractive pairs need no special table.
//
// IV, IX, XL, XC, CD, CM look like six exceptions, but they share one
// property: a smaller symbol standing in front of a larger one. Roman numerals
// otherwise never increase left to right, so that comparison identifies every
// subtractive pair and nothing else. Subtract when it holds, add when it does
// not, and the pairs take care of themselves.

fn value_of(c) {
    if c == "I" { 1 } else if c == "V" { 5 } else if c == "X" { 10 }
    else if c == "L" { 50 } else if c == "C" { 100 } else if c == "D" { 500 }
    else if c == "M" { 1000 } else { 0 }
}

fn trace(input) {
    let s = input.s;
    let cs = chars(s);
    let n = cs.len();

    enter("romanToInt", #{});

    let table = map_new();
    for c in ["I", "V", "X", "L", "C", "D", "M"] { map_set(table, c, value_of(c)); }
    step("table", "Each symbol has one fixed value; the pairs are not listed anywhere.",
        [ array("numeral", cs), kv("symbol values", table) ]);

    let total = 0;
    set(#{ total: N(total) });
    step("init", "Running total.",
        [ array("numeral", cs), text("total", [`${total}`]) ]);

    for i in 0..n {
        let v = value_of(cs[i]);
        step("loop", `Symbol ${i}: "${cs[i]}".`,
            [ array("numeral", cs).ptr("i", i).done(before(i)), text("total", [`${total}`]) ]);

        step("lookup", `"${cs[i]}" is worth ${v}.`,
            [ array("numeral", cs).ptr("i", i).hl([i]),
              kv("symbol values", table).hl([cs[i]]) ]);

        if i + 1 < n && v < value_of(cs[i + 1]) {
            step("smaller", `The next symbol "${cs[i + 1]}" is worth ${value_of(cs[i + 1])}, which is more. Roman numerals never increase left to right unless it is a subtractive pair — so this is one.`,
                [ array("numeral", cs).ptr("i", i).hl([i, i + 1]) ]);
            total -= v;
            set(#{ total: N(total) });
            log(`${cs[i]}${cs[i + 1]} is subtractive: -${v}`);
            step("subtract", `Subtract ${v}; the pair's real value arrives when "${cs[i + 1]}" is added next. Total ${total}.`,
                [ array("numeral", cs).ptr("i", i).bad([i]), text("total", [`${total}`]) ]);
        } else {
            step("bigger",
                if i + 1 < n {
                    `The next symbol "${cs[i + 1]}" is worth ${value_of(cs[i + 1])}, which is no more — an ordinary, additive symbol.`
                } else {
                    "It is the last symbol, so nothing can make it subtractive.".to_string()
                },
                [ array("numeral", cs).ptr("i", i).hl([i]) ]);
            total += v;
            set(#{ total: N(total) });
            step("add", `Add ${v}. Total ${total}.`,
                [ array("numeral", cs).ptr("i", i).done(before(i + 1)),
                  text("total", [`${total}`]) ]);
        }
    }

    log(`answer ${total}`, "result");
    result(`${total}`);
    step("ret", `${s} = ${total}.`,
        [ array("numeral", cs).done(seq(0, n - 1)), text("total", [`${total}`]) ], "return");
    leave(`${total}`);
}
