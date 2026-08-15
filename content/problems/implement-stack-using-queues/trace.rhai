// Implement Stack Using Queues — make push pay so the reads stay free.
//
// A queue returns its oldest element; a stack wants its newest. Something has
// to bridge that gap, and the choice is *where* to spend the work. Rotating
// the older elements around behind the new one, once per push, leaves the
// queue permanently in stack order — so pop and top do no work at all.

fn trace(input) {
    let q = [];
    let out = [];

    enter("MyStack", #{});
    step("init", "One queue. Its front will always hold the value a stack would return next.",
        [ queue("queue (front is leftmost)", q) ]);

    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() == 0 { continue; }
        let cmd = parts[0];

        if cmd == "push" {
            let x = num(parts[1]);
            enter(`push(${x})`, #{ x: N(x) });
            q.push(x);
            step("enqueue", `Enqueue ${x}. It is at the *back*, which is the wrong end for a stack.`,
                [ queue("queue (front is leftmost)", q) ]);

            let moves = q.len() - 1;
            step("rotate", `Rotate the ${plural(moves, "older element")} around behind it.`,
                [ queue("queue (front is leftmost)", q) ]);

            for k in 0..moves {
                let head = q[0];
                q.remove(0);
                q.push(head);
                step("moveBack", `Take ${head} off the front and put it at the back.`,
                    [ queue("queue (front is leftmost)", q) ]);
            }

            log(`push ${x}; front is now ${q[0]}`);
            step("rotate", `Rotation done: ${x} is at the front, so it is the next thing any read returns.`,
                [ queue("queue (front is leftmost)", q) ], "return");
            leave("");
        } else if cmd == "pop" {
            enter("pop()", #{});
            let v = q[0];
            q.remove(0);
            out.push(`${v}`);
            log(`pop -> ${v}`, "result");
            step("pop", `The front is ${v} — the most recently pushed value, because push already arranged that.`,
                [ queue("queue (front is leftmost)", q), text("output", out) ]);
            step("retPop", `Return ${v}. No rearranging needed here at all.`,
                [ queue("queue (front is leftmost)", q), text("output", out) ], "return");
            leave(`${v}`);
        } else if cmd == "top" {
            enter("top()", #{});
            let v = q[0];
            out.push(`${v}`);
            log(`top -> ${v}`, "result");
            step("top", `Read the front without removing it: ${v}.`,
                [ queue("queue (front is leftmost)", q), text("output", out) ], "return");
            leave(`${v}`);
        } else {
            enter("empty()", #{});
            let e = q.len() == 0;
            out.push(`${e}`);
            log(`empty -> ${e}`, "result");
            step("empty", `The queue holds ${q.len()} value(s), so empty() is ${e}.`,
                [ queue("queue (front is leftmost)", q), text("output", out) ], "return");
            leave(`${e}`);
        }
    }

    let answer = join(out, " ");
    result(answer);
    leave(answer);
}
