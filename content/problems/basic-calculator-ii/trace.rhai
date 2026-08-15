// Basic Calculator II — precedence without a parse tree.
//
// The stack holds terms that are only ever summed at the end, so * and / must
// be resolved the moment they are known: they reach back and rewrite the top
// of the stack. + and − simply push (a minus pushes a negative), which defers
// them to the final sum. That asymmetry *is* operator precedence, expressed in
// four lines.
//
// The other trick is timing: a number is not acted on when it is read, but
// when the next operator arrives — because only then is it known which
// operator applies to it.

const ST = "stack — terms still to be added";

fn trace(input) {
    let cs = chars(input.s);
    let n = cs.len();

    enter("calculate", #{ s: S(input.s) });

    let stack = [];
    step("init", "One stack of terms. Nothing here understands precedence; the *timing* of the pushes does.",
        [ array("s", cs), stack(global::ST, stack) ]);

    let num = 0;
    let op = "+";
    set(#{ num: N(0), op: S(op) });
    step("state", "The pending operator starts as + so the first number is simply pushed.",
        [ text("pending", ["op = +", "num = 0"]) ]);

    for i in 0..n {
        let c = cs[i];
        set(#{ i: N(i), c: S(c) });
        step("ch", `Character ${i}: '${c}'.`,
            [ array("s", cs).ptr("i", i).done(before(i)), stack(global::ST, stack) ]);

        let is_digit = c >= "0" && c <= "9";
        if is_digit {
            num = num * 10 + (ord(c) - ord("0"));
            set(#{ num: N(num) });
            step("digit", `A digit — it may be part of a longer number.`,
                [ array("s", cs).ptr("i", i).hl([i]) ]);
            step("build", `num = ${num}. Multi-digit numbers are built here, one digit at a time.`,
                [ text("pending", [`op = ${op}`, `num = ${num}`]) ]);
        }

        if (c != " " && !is_digit) || i == n - 1 {
            step("flush", `${if is_digit { "End of the string" } else { `'${c}' is an operator` }} — so the number just built, ${num}, finally has an operator waiting for it: ${op}.`,
                [ array("s", cs).ptr("i", i), stack(global::ST, stack),
                  text("pending", [`op = ${op}`, `num = ${num}`]) ]);
            step("apply", `Apply the *pending* operator ${op}, not the one just read. That one applies to the next number.`,
                [ stack(global::ST, stack) ]);

            if op == "+" {
                stack.push(num);
                step("plus", `Push +${num}. Addition can wait until the end, so it just joins the pile.`,
                    [ stack(global::ST, stack).pushed(true) ]);
            } else if op == "-" {
                stack.push(-num);
                step("minus", `Push −${num}. Subtraction is addition of a negative — no special case at the end.`,
                    [ stack(global::ST, stack).pushed(true) ]);
            } else if op == "*" {
                let top = stack[stack.len() - 1];
                stack[stack.len() - 1] = top * num;
                step("times", `${top} × ${num} = ${stack[stack.len() - 1]} — folded into the top of the stack immediately, because × binds tighter than the + or − that put it there.`,
                    [ stack(global::ST, stack).hl([stack.len() - 1]) ]);
            } else {
                let top = stack[stack.len() - 1];
                let q = top / num;
                stack[stack.len() - 1] = q;
                step("divide", `${top} ÷ ${num} = ${q}, truncated toward zero, folded into the top of the stack.`,
                    [ stack(global::ST, stack).hl([stack.len() - 1]) ]);
            }

            op = c;
            num = 0;
            set(#{ op: S(op), num: N(0) });
            step("remember", `Remember '${c}' as the operator for whatever number comes next.`,
                [ text("pending", [`op = ${op}`, "num = 0"]) ]);
        }
    }

    let sum = 0;
    set(#{ sum: N(0) });
    step("sum", "Every term on the stack is now safe to add — all the higher-precedence work has already been folded in.",
        [ stack(global::ST, stack) ]);

    for k in 0..stack.len() {
        sum += stack[k];
        set(#{ sum: N(sum) });
        step("add", `+ ${stack[k]} → ${sum}.`,
            [ stack(global::ST, stack).hl([k]), text("total", [`sum = ${sum}`]) ]);
    }

    log(`= ${sum}`, "result");
    result(`${sum}`);
    step("ret", `${input.s} = ${sum}.`, [ text("total", [`sum = ${sum}`]) ], "return");
    leave(`${sum}`);
}
