// Min Stack — a second stack that remembers the minimum at each depth.
//
// getMin has to be O(1), so it cannot scan. Pushing "the minimum as of this
// depth" alongside every value means popping restores the previous minimum for
// free — the history is already there.

fn trace(input) {
    let tokens = words(input.ops);
    let stack = [];
    let mins = [];
    let out = [];

    enter("MinStack", #{});
    set(#{ stack: A(stack) });
    step("initS", "The value stack.", [ stack("stack", stack) ]);
    set(#{ mins: A(mins) });
    step("initM", "The parallel stack: mins[i] is the smallest value among the first i+1 pushes.",
        [ stack("stack", stack), stack("mins", mins) ]);
    step("initOut", "Collect the answers the operations produce.",
        [ stack("stack", stack), stack("mins", mins) ]);

    let i = 0;
    while i < tokens.len() {
        let op = tokens[i];
        set(#{ op: S(op) });
        step("loop", `Operation: ${op}.`,
            [ stack("stack", stack), stack("mins", mins), text("output", out) ]);

        if op == "push" {
            i += 1;
            let x = num(tokens[i]);
            step("isPush", `push(${x}).`,
                [ stack("stack", stack), stack("mins", mins) ]);
            stack.push(x);
            set(#{ stack: A(stack) });
            step("push", `${x} goes on the value stack.`,
                [ stack("stack", stack).pushed(true), stack("mins", mins) ]);

            let m = x;
            if mins.len() > 0 && mins[mins.len() - 1] < m {
                m = mins[mins.len() - 1];
                step("cmpMin", `The previous minimum ${m} is smaller than ${x}, so it stays the minimum.`,
                    [ stack("stack", stack), stack("mins", mins) ]);
            } else {
                step("cmpMin", `${x} is the smallest value in the stack so far.`,
                    [ stack("stack", stack), stack("mins", mins) ]);
            }
            mins.push(m);
            set(#{ mins: A(mins) });
            step("pushMin", `Record ${m} at this depth. Both stacks always have the same height.`,
                [ stack("stack", stack), stack("mins", mins).pushed(true) ]);
        } else if op == "pop" {
            step("isPop", "pop().", [ stack("stack", stack), stack("mins", mins) ]);
            let v = stack[stack.len() - 1];
            stack.truncate(stack.len() - 1);
            set(#{ stack: A(stack) });
            step("pop", `Remove ${v}.`,
                [ stack("stack", stack).popped(v), stack("mins", mins) ]);
            let mv = mins[mins.len() - 1];
            mins.truncate(mins.len() - 1);
            set(#{ mins: A(mins) });
            step("popMin", `Drop its minimum too — the one underneath is automatically correct again.`,
                [ stack("stack", stack), stack("mins", mins).popped(mv) ]);
            out.push(`${v}`);
            step("emit", `Report ${v}.`,
                [ stack("stack", stack), stack("mins", mins), text("output", out) ]);
        } else if op == "top" {
            step("isTop", "top().", [ stack("stack", stack), stack("mins", mins) ]);
            out.push(`${stack[stack.len() - 1]}`);
            log(`top -> ${stack[stack.len() - 1]}`, "result");
            step("emit2", `The top value is ${stack[stack.len() - 1]}.`,
                [ stack("stack", stack), stack("mins", mins), text("output", out) ]);
        } else {
            step("isMin", "getMin().", [ stack("stack", stack), stack("mins", mins) ]);
            out.push(`${mins[mins.len() - 1]}`);
            log(`getMin -> ${mins[mins.len() - 1]}`, "result");
            step("emit3", `The answer is sitting on top of the mins stack: ${mins[mins.len() - 1]}. No scanning.`,
                [ stack("stack", stack), stack("mins", mins), text("output", out) ]);
        }
        i += 1;
    }

    let answer = join(out, " ");
    result(answer);
    step("ret", `All operations answered in O(1): ${answer}.`,
        [ stack("stack", stack), stack("mins", mins), text("output", out) ], "return");
    leave(answer);
}
