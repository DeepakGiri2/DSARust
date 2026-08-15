// Implement Queue Using Stacks — pouring a stack reverses it.
//
// Tip one stack into another and the order flips; a flipped stack is a queue.
// The subtlety is *when* to pour. Pouring on every operation would be slow and
// wrong; pouring only when the out-stack is empty means each element crosses
// exactly once in its life, which is what makes the amortised cost constant
// even though one individual pop can move everything.

fn trace(input) {
    let inbox = [];
    let outbox = [];
    let out = [];

    enter("MyQueue", #{});
    step("init", "Arrivals pile up on the in-stack; departures are served from the out-stack, which holds them reversed.",
        [ stack("in-stack", inbox), stack("out-stack", outbox) ]);

    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() == 0 { continue; }
        let cmd = parts[0];

        if cmd == "push" {
            let x = num(parts[1]);
            enter(`push(${x})`, #{ x: N(x) });
            inbox.push(x);
            log(`push ${x}`);
            step("push", `Push ${x} onto the in-stack and do nothing else — arrivals are always cheap.`,
                [ stack("in-stack", inbox).pushed(true), stack("out-stack", outbox) ], "return");
            leave("");
            continue;
        }

        if cmd == "empty" {
            enter("empty()", #{});
            let e = inbox.len() == 0 && outbox.len() == 0;
            out.push(`${e}`);
            log(`empty -> ${e}`, "result");
            step("empty", `Both stacks would have to be empty; they hold ${inbox.len()} and ${outbox.len()}, so empty() is ${e}.`,
                [ stack("in-stack", inbox), stack("out-stack", outbox), text("output", out) ], "return");
            leave(`${e}`);
            continue;
        }

        // pop and peek share the transfer.
        let reading = if cmd == "pop" { "pop" } else { "peek" };
        enter(`${reading}()`, #{});
        if reading == "pop" {
            step("callPop", "Before serving anything, make sure the out-stack has something in it.",
                [ stack("in-stack", inbox), stack("out-stack", outbox) ], "call");
        } else {
            step("callPeek", "Peek needs the same guarantee as pop: the out-stack must be non-empty.",
                [ stack("in-stack", inbox), stack("out-stack", outbox) ], "call");
        }

        enter("transfer()", #{});
        if outbox.len() == 0 {
            step("needTransfer", `The out-stack is empty, so the ${plural(inbox.len(), "waiting arrival")} must be poured across.`,
                [ stack("in-stack", inbox), stack("out-stack", outbox) ]);
            while inbox.len() > 0 {
                let v = inbox[inbox.len() - 1];
                step("drain", `The in-stack still has ${inbox.len()} value(s); its top is ${v}, which is the *newest* arrival.`,
                    [ stack("in-stack", inbox), stack("out-stack", outbox) ]);
                inbox.truncate(inbox.len() - 1);
                step("move", `Take ${v} off the in-stack.`,
                    [ stack("in-stack", inbox).popped(v), stack("out-stack", outbox) ]);
                outbox.push(v);
                step("onto", `Put it on the out-stack. Newest goes deepest, so the oldest ends up on top — that is the reversal.`,
                    [ stack("in-stack", inbox), stack("out-stack", outbox).pushed(true) ]);
            }
            step("drain", "The in-stack is empty; the pour is done.",
                [ stack("in-stack", inbox), stack("out-stack", outbox) ], "return");
        } else {
            step("needTransfer", "The out-stack still holds values, so nothing is poured — pouring now would put newer arrivals in front of older ones.",
                [ stack("in-stack", inbox), stack("out-stack", outbox) ], "return");
        }
        leave("");

        let v = outbox[outbox.len() - 1];
        if reading == "pop" {
            outbox.truncate(outbox.len() - 1);
            out.push(`${v}`);
            log(`pop -> ${v}`, "result");
            step("pop", `The out-stack's top is ${v} — the oldest value still waiting.`,
                [ stack("in-stack", inbox), stack("out-stack", outbox).popped(v), text("output", out) ]);
            step("retPop", `Return ${v}. Every value crosses between the stacks exactly once, so this is O(1) on average.`,
                [ stack("in-stack", inbox), stack("out-stack", outbox), text("output", out) ], "return");
        } else {
            out.push(`${v}`);
            log(`peek -> ${v}`, "result");
            step("peek", `Read the out-stack's top without removing it: ${v}.`,
                [ stack("in-stack", inbox), stack("out-stack", outbox), text("output", out) ], "return");
        }
        leave(`${v}`);
    }

    let answer = join(out, " ");
    result(answer);
    leave(answer);
}
