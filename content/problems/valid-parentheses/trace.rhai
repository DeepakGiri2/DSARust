// Valid Parentheses — the stack remembers what is still open.
//
// Reading the animation, the stack should look like the nesting itself: it
// grows as brackets open and drains as each one is closed by its own kind.

fn is_open(c) {
    c == "(" || c == "[" || c == "{"
}

fn opener_for(c) {
    if c == ")" { "(" } else if c == "]" { "[" } else { "{" }
}

fn trace(input) {
    let s = input.s;
    let cs = chars(s);
    let stack = [];

    enter("isValid", #{ s: S(s) });
    set(#{ stack: A(stack) });
    step("init", "Start with an empty stack. It will hold every bracket that is open but not yet closed.",
        [ array("s", cs), stack("stack", stack) ]);

    for i in 0..cs.len() {
        let c = cs[i];
        set(#{ i: N(i), c: S(c) });
        step("loop", `i = ${i}: the character is '${c}'.`,
            [ array("s", cs).ptr("i", i).done(before(i)), stack("stack", stack) ]);

        if is_open(c) {
            step("isopen", `'${c}' opens something, so it has to be remembered until its partner shows up.`,
                [ array("s", cs).ptr("i", i).hl([i]).done(before(i)), stack("stack", stack) ]);
            stack.push(c);
            set(#{ stack: A(stack) });
            log(`push '${c}' → stack depth ${stack.len()}`);
            step("push", `Push '${c}'. The stack is ${stack.len()} deep — that is how many brackets are still open.`,
                [ array("s", cs).ptr("i", i).done(before(i)),
                  stack("stack", stack).pushed(true) ]);
        } else {
            step("isopen", `'${c}' is a closer, so it must match whatever was opened most recently.`,
                [ array("s", cs).ptr("i", i).done(before(i)), stack("stack", stack) ]);

            if stack.len() == 0 {
                step("empty", `The stack is empty — there is nothing for '${c}' to close.`,
                    [ array("s", cs).ptr("i", i).bad([i]).done(before(i)), stack("stack", stack).bad([]) ]);
                log(`'${c}' with an empty stack → invalid`, "result");
                result("false");
                step("bad", "Return false: a closing bracket with nothing open before it.",
                    [ array("s", cs).bad([i]) ], "return");
                leave("false");
                return;
            }

            let top = stack[stack.len() - 1];
            set(#{ top: S(top) });
            step("peek", `The most recently opened bracket is '${top}'.`,
                [ array("s", cs).ptr("i", i).done(before(i)),
                  stack("stack", stack) ]);

            let want = opener_for(c);
            if top != want {
                step("match", `'${c}' needs '${want}' on top, but the top is '${top}' — the nesting is crossed.`,
                    [ array("s", cs).ptr("i", i).bad([i]).done(before(i)),
                      stack("stack", stack).bad([]) ]);
                log(`expected '${want}', found '${top}' → invalid`, "result");
                result("false");
                step("bad2", "Return false: brackets closed in the wrong order.",
                    [ array("s", cs).bad([i]), stack("stack", stack).bad([]) ], "return");
                leave("false");
                return;
            }

            step("match", `'${c}' matches the '${top}' on top. This pair is complete.`,
                [ array("s", cs).ptr("i", i).hl([i]).done(before(i)),
                  stack("stack", stack) ]);
            let popped = stack.pop();
            set(#{ stack: A(stack) });
            log(`pop '${popped}' → stack depth ${stack.len()}`);
            step("pop", `Pop '${popped}'. ${stack.len()} bracket(s) still open.`,
                [ array("s", cs).ptr("i", i).done(before(i)),
                  stack("stack", stack).popped(popped) ]);
        }
    }

    let ok = stack.len() == 0;
    log(`end of string, stack depth ${stack.len()} → ${ok}`, "result");
    result(`${ok}`);
    step("end",
        if ok {
            "Everything that opened was closed and the stack is empty — the string is valid."
        } else {
            `The string ended with ${stack.len()} bracket(s) still open, so it is not valid.`
        },
        [ array("s", cs).done(seq(0, cs.len() - 1)), stack("stack", stack) ],
        "return");
    leave(`${ok}`);
}
