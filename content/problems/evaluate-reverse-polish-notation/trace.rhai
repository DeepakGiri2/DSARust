// Evaluate Reverse Polish Notation — a stack of operands.
//
// RPN needs no parentheses because an operator always applies to the two
// values immediately before it. A stack captures exactly that "most recent
// two" rule, which is why the whole evaluation is one pass.

fn is_op(t) {
    t == "+" || t == "-" || t == "*" || t == "/"
}

fn apply(a, b, op) {
    if op == "+" { a + b } else if op == "-" { a - b }
    else if op == "*" { a * b } else { a / b }
}

fn trace(input) {
    let tokens = words(input.tokens);
    let stack = [];

    enter("evalRPN", #{ tokens: S(input.tokens) });
    set(#{ stack: A(stack) });
    step("init", "A stack of operands waiting for an operator to claim them.",
        [ array("tokens", tokens), stack("stack", stack) ]);

    for i in 0..tokens.len() {
        let t = tokens[i];
        step("loop", `Token ${i}: "${t}".`,
            [ array("tokens", tokens).ptr("i", i).done(before(i)), stack("stack", stack) ]);

        if is_op(t) {
            step("isop", `"${t}" is an operator, so it applies to the two most recent values.`,
                [ array("tokens", tokens).ptr("i", i).hl([i]), stack("stack", stack) ]);

            let b = stack[stack.len() - 1];
            stack.truncate(stack.len() - 1);
            set(#{ b: N(b) });
            step("popB", `Pop ${b}. It was pushed last, so it is the right-hand operand.`,
                [ array("tokens", tokens).ptr("i", i), stack("stack", stack).popped(b) ]);

            let a = stack[stack.len() - 1];
            stack.truncate(stack.len() - 1);
            set(#{ a: N(a) });
            step("popA", `Pop ${a} — the left-hand operand.`,
                [ array("tokens", tokens).ptr("i", i), stack("stack", stack).popped(a) ]);

            let r = apply(a, b, t);
            stack.push(r);
            set(#{ stack: A(stack) });
            log(`${a} ${t} ${b} = ${r}`);
            step("push", `${a} ${t} ${b} = ${r}. Push it back — a result is an operand like any other.`,
                [ array("tokens", tokens).ptr("i", i).done(before(i + 1)),
                  stack("stack", stack).pushed(true) ]);
        } else {
            step("parse", `"${t}" is a number.`,
                [ array("tokens", tokens).ptr("i", i).hl([i]), stack("stack", stack) ]);
            stack.push(num(t));
            set(#{ stack: A(stack) });
            step("pushNum", `Push ${t} and wait.`,
                [ array("tokens", tokens).ptr("i", i).done(before(i + 1)),
                  stack("stack", stack).pushed(true) ]);
        }
    }

    log(`answer ${stack[0]}`, "result");
    result(`${stack[0]}`);
    step("ret", `One value remains on the stack, and that is the answer: ${stack[0]}.`,
        [ array("tokens", tokens).done(seq(0, tokens.len() - 1)), stack("stack", stack) ], "return");
    leave(`${stack[0]}`);
}
