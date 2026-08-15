// Multiply Strings — the position of a partial product is arithmetic, not
// bookkeeping.
//
// School long multiplication writes each partial product on its own line and
// then adds the lines up. That second phase is unnecessary once you notice
// where a product must land: digit i of the first number and digit j of the
// second contribute to place i + j + 1, with any carry going to i + j. Adding
// straight into one array and normalising each cell as it is written removes
// both the extra lines and the final carry sweep.

fn digits_of(s) {
    let out = [];
    for c in chars(s) { out.push(num(c)); }
    out
}

fn trace(input) {
    let a = input.num1;
    let b = input.num2;

    enter("multiply", #{});

    if a == "0" || b == "0" {
        step("zero", "One factor is zero, so the product is zero — and the digit loop would otherwise produce a string of leading zeros.",
            [ text("factors", [`num1 = ${a}`, `num2 = ${b}`]) ]);
        result("0");
        step("ret", "Return \"0\".", [ text("answer", ["0"]) ], "return");
        leave("0");
        return;
    }
    step("zero", "Neither factor is zero.",
        [ text("factors", [`num1 = ${a}`, `num2 = ${b}`]) ]);

    let da = digits_of(a);
    let db = digits_of(b);
    let m = da.len();
    let n = db.len();

    let res = [];
    for k in 0..(m + n) { res.push(0); }
    step("init", `A product of an ${m}-digit and an ${n}-digit number needs at most ${m + n} digits, so one array of that size can hold every partial product at once.`,
        [ array("num1", da), array("num2", db), array("result digits", res) ]);

    let i = m - 1;
    while i >= 0 {
        step("outer", `Take digit ${i} of num1: ${da[i]}.`,
            [ array("num1", da).ptr("i", i).hl([i]), array("num2", db),
              array("result digits", res) ]);

        let j = n - 1;
        while j >= 0 {
            step("inner", `Against digit ${j} of num2: ${db[j]}.`,
                [ array("num1", da).ptr("i", i).hl([i]),
                  array("num2", db).ptr("j", j).hl([j]),
                  array("result digits", res) ]);

            let p = da[i] * db[j];
            step("mul", `${da[i]} × ${db[j]} = ${p}.`,
                [ array("result digits", res).hl([i + j, i + j + 1]),
                  text("placement", [`i = ${i}, j = ${j}`,
                                     `low digit goes to position ${i + j + 1}`,
                                     `carry goes to position ${i + j}`]) ]);

            let total = p + res[i + j + 1];
            step("place", `Position ${i + j + 1} already holds ${res[i + j + 1]}, so the running total there is ${total}.`,
                [ array("result digits", res).ptr("here", i + j + 1).hl([i + j + 1]) ]);

            res[i + j + 1] = total % 10;
            step("digit", `Keep ${total % 10} there.`,
                [ array("result digits", res).ptr("here", i + j + 1).hl([i + j + 1]) ]);

            res[i + j] = res[i + j] + total / 10;
            step("carry", `Carry ${total / 10} into position ${i + j}, which now holds ${res[i + j]}. Normalising immediately is why no carry sweep is needed at the end.`,
                [ array("result digits", res).ptr("carry", i + j).hl([i + j]) ]);

            j -= 1;
        }
        i -= 1;
    }

    step("build", "Every pair of digits has contributed. What remains is to read the array off as text.",
        [ array("result digits", res) ]);

    let out = "";
    for k in 0..res.len() {
        if out == "" && res[k] == 0 && k + 1 < res.len() {
            step("trim", `Position ${k} is a leading zero — the array is sized for the worst case, and this product did not need it.`,
                [ array("result digits", res).ptr("k", k).bad([k]) ]);
            continue;
        }
        out += `${res[k]}`;
        step("trim", `Append ${res[k]}: "${out}".`,
            [ array("result digits", res).ptr("k", k).hl([k]), text("building", [out]) ]);
    }

    log(`answer ${out}`, "result");
    result(out);
    step("ret", `${a} × ${b} = ${out}.`, [ text("answer", [out]) ], "return");
    leave(out);
}
