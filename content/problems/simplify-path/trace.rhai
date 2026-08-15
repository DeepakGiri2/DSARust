// Simplify Path — "go up one level" is literally a pop.
//
// Splitting on "/" turns the path into a sequence of instructions, and a stack
// of directory names is the state they act on. The only case that needs care
// is ".." at the root: there is nothing above "/", so the pop is skipped
// rather than being an error.

fn trace(input) {
    let path = input.path;
    let parts = path.split("/");
    let stack = [];

    enter("simplifyPath", #{ path: S(path) });
    step("init", `Split on "/" — ${parts.len()} piece(s), some of which are empty because of repeated or trailing slashes.`,
        [ array("pieces", show_parts(parts)), stack("directories", stack) ]);

    for i in 0..parts.len() {
        let part = parts[i];
        step("loop", `Piece ${i}: ${quoted(part)}.`,
            [ array("pieces", show_parts(parts)).ptr("i", i).done(before(i)),
              stack("directories", stack) ]);

        if part == "" || part == "." {
            step("skip",
                if part == "" {
                    "An empty piece comes from a doubled or trailing slash — it names nothing, so it changes nothing.".to_string()
                } else {
                    `"." means "this directory", so it changes nothing either.`
                },
                [ array("pieces", show_parts(parts)).ptr("i", i),
                  stack("directories", stack) ]);
            continue;
        }

        if part == ".." {
            step("up", `".." means leave the directory just entered.`,
                [ array("pieces", show_parts(parts)).ptr("i", i).hl([i]),
                  stack("directories", stack) ]);

            if stack.len() > 0 {
                let gone = stack[stack.len() - 1];
                step("canPop", `There is somewhere to go up from: currently inside "${gone}".`,
                    [ array("pieces", show_parts(parts)).ptr("i", i),
                      stack("directories", stack) ]);
                stack.truncate(stack.len() - 1);
                log(`leave ${gone}`);
                step("pop", `Pop it — going up one level *is* undoing the last push.`,
                    [ array("pieces", show_parts(parts)).ptr("i", i),
                      stack("directories", stack).popped(gone) ]);
            } else {
                step("canPop", "Already at the root, and there is nothing above it — so this piece is simply ignored rather than being an error.",
                    [ array("pieces", show_parts(parts)).ptr("i", i),
                      stack("directories", stack) ]);
            }
            continue;
        }

        stack.push(part);
        log(`enter ${part}`);
        step("push", `"${part}" is a real directory name; enter it.`,
            [ array("pieces", show_parts(parts)).ptr("i", i).done(before(i + 1)),
              stack("directories", stack).pushed(true) ]);
    }

    let answer = "/" + join(stack, "/");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Join what survived, under a leading slash: ${answer}.`,
        [ stack("directories", stack), text("canonical path", [answer]) ], "return");
    leave(answer);
}

fn show_parts(parts) {
    let out = [];
    for p in parts { out.push(if p == "" { "∅" } else { p }); }
    out
}

fn quoted(p) {
    if p == "" { "an empty piece" } else { `"${p}"` }
}
