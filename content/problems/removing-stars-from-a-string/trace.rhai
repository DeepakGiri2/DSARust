// Removing Stars — "closest surviving character to the left" is a stack top.
//
// Read literally, each star triggers a search leftwards past everything
// already deleted. But if the answer is built on a stack, that search has
// already been done: whatever is on top is by construction the nearest
// character still standing. Every star is then one pop.

fn trace(input) {
    let s = input.s;
    let cs = chars(s);
    let stack = [];

    enter("removeStars", #{});
    step("init", "The surviving characters, newest on top.",
        [ array("input", cs), stack("survivors", stack) ]);

    for i in 0..cs.len() {
        let ch = cs[i];
        step("loop", `Character ${i}: "${ch}".`,
            [ array("input", cs).ptr("i", i).done(before(i)),
              stack("survivors", stack) ]);

        if ch == "*" {
            let gone = stack[stack.len() - 1];
            step("isStar", `A star. The nearest surviving character to its left is "${gone}" — and it is already on top, so nothing has to be searched for.`,
                [ array("input", cs).ptr("i", i).bad([i]), stack("survivors", stack) ]);
            stack.truncate(stack.len() - 1);
            log(`star removes "${gone}"`);
            step("pop", `Remove "${gone}" and the star with it.`,
                [ array("input", cs).ptr("i", i), stack("survivors", stack).popped(gone) ]);
        } else {
            step("isChar", `"${ch}" is an ordinary character, so it survives for now.`,
                [ array("input", cs).ptr("i", i).hl([i]), stack("survivors", stack) ]);
            stack.push(ch);
            step("push", `Push it — it is now the one a later star would delete.`,
                [ array("input", cs).ptr("i", i).done(before(i + 1)),
                  stack("survivors", stack).pushed(true) ]);
        }
    }

    let answer = join(stack, "");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `What is left, bottom to top, is the answer: "${answer}".`,
        [ stack("survivors", stack), text("result", [answer]) ], "return");
    leave(answer);
}
