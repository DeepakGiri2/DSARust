// Design Circular Queue — head plus count, never head plus tail.
//
// With two pointers, head == tail is ambiguous: it means empty *and* it means
// full, and every implementation that stores both ends up wasting a slot or
// carrying a boolean to break the tie. Storing the count instead makes the
// tail a derived value, both questions trivial, and the wrap-around a single
// modulo. Watch the head pointer walk off the end and reappear at index 0.

const BUF = "buffer";

fn parse_ops(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if !w.is_empty() { out.push(w); }
    }
    out
}

fn slots(k, head, count) {
    // The indices currently holding live elements, in queue order.
    let out = [];
    for i in 0..count { out.push((head + i) % k); }
    out
}

fn state_lines(k, head, count) {
    [`head  = ${head}`, `count = ${count} of ${k}`,
     if count == 0 { "tail  = —" } else { `tail  = ${(head + count - 1) % k}` }]
}

fn trace(input) {
    let k = input.k;
    let ops = parse_ops(input.ops);

    enter("MyCircularQueue", #{ k: N(k) });

    let buf = [];
    for i in 0..k { buf.push(0); }
    let head = 0;
    let count = 0;
    set(#{ head: N(0), count: N(0) });
    step("init", `A ${k}-slot array, and that is the entire allocation — nothing is ever resized or shifted.`,
        [ array(global::BUF, buf), text("state", state_lines(k, head, count)) ]);
    leave("ready");

    let out = [];
    for o in ops {
        let name = o[0];

        if name == "enQueue" {
            let v = num(o[1]);
            enter(`enQueue(${v})`, #{ value: N(v) });
            step("enq", `enQueue(${v}).`,
                [ array(global::BUF, buf).ptr("head", head).done(slots(k, head, count)),
                  text("state", state_lines(k, head, count)) ]);

            if count == k {
                out.push("false");
                step("full", `count is ${count} and the buffer holds ${k} — there is nowhere to put ${v}.`,
                    [ array(global::BUF, buf).bad(slots(k, head, count)),
                      text("state", state_lines(k, head, count)) ]);
                step("nofit", "Return false. A circular queue is fixed size: it refuses, it does not grow.",
                    [ text("state", state_lines(k, head, count)) ], "return");
                leave("false");
                continue;
            }

            let tail = (head + count) % k;
            set(#{ tail: N(tail) });
            step("tail", `The next free slot is (head + count) mod k = (${head} + ${count}) mod ${k} = ${tail}. The tail is computed, never stored.`,
                [ array(global::BUF, buf).ptr("head", head).ptr("tail", tail).done(slots(k, head, count)),
                  text("state", state_lines(k, head, count)) ]);

            buf[tail] = v;
            step("store", `buf[${tail}] = ${v}.`,
                [ array(global::BUF, buf).ptr("head", head).hl([tail]).done(slots(k, head, count)) ]);

            count += 1;
            set(#{ count: N(count) });
            out.push("true");
            step("grew", `count = ${count}. Return true.`,
                [ array(global::BUF, buf).ptr("head", head).done(slots(k, head, count)),
                  text("state", state_lines(k, head, count)) ], "return");
            leave("true");

        } else if name == "deQueue" {
            enter("deQueue");
            step("deq", "deQueue().",
                [ array(global::BUF, buf).ptr("head", head).done(slots(k, head, count)),
                  text("state", state_lines(k, head, count)) ]);

            if count == 0 {
                out.push("false");
                step("empty", "count is 0 — nothing to remove.",
                    [ text("state", state_lines(k, head, count)) ]);
                step("noitem", "Return false.", [ text("state", state_lines(k, head, count)) ], "return");
                leave("false");
                continue;
            }

            let gone = buf[head];
            head = (head + 1) % k;
            set(#{ head: N(head) });
            step("advance", `Drop ${gone} by moving head to (${(head + k - 1) % k} + 1) mod ${k} = ${head}. The cell is not cleared — it is simply no longer inside the queue.`,
                [ array(global::BUF, buf).ptr("head", head).done(slots(k, head, count - 1)) ]);

            count -= 1;
            set(#{ count: N(count) });
            out.push("true");
            step("shrank", `count = ${count}. Return true.`,
                [ array(global::BUF, buf).ptr("head", head).done(slots(k, head, count)),
                  text("state", state_lines(k, head, count)) ], "return");
            leave("true");

        } else if name == "Front" {
            enter("Front");
            step("front", "Front().",
                [ array(global::BUF, buf).ptr("head", head).done(slots(k, head, count)) ]);
            if count == 0 {
                out.push("-1");
                step("fnone", "Empty — return -1.", [ text("state", state_lines(k, head, count)) ], "return");
                leave("-1");
            } else {
                out.push(`${buf[head]}`);
                step("fval", `The front element is buf[head] = buf[${head}] = ${buf[head]}.`,
                    [ array(global::BUF, buf).ptr("head", head).hl([head]).done(slots(k, head, count)) ], "return");
                leave(`${buf[head]}`);
            }

        } else if name == "Rear" {
            enter("Rear");
            step("rear", "Rear().",
                [ array(global::BUF, buf).ptr("head", head).done(slots(k, head, count)) ]);
            if count == 0 {
                out.push("-1");
                step("rnone", "Empty — return -1.", [ text("state", state_lines(k, head, count)) ], "return");
                leave("-1");
            } else {
                let idx = (head + count - 1) % k;
                out.push(`${buf[idx]}`);
                step("rval", `The last element is at (head + count − 1) mod k = ${idx}: ${buf[idx]}.`,
                    [ array(global::BUF, buf).ptr("head", head).hl([idx]).done(slots(k, head, count)) ], "return");
                leave(`${buf[idx]}`);
            }

        } else if name == "isEmpty" {
            enter("isEmpty");
            out.push(if count == 0 { "true" } else { "false" });
            step("isempty", `count == 0 is ${count == 0} — no ambiguity, because the count is stored rather than inferred from two pointers.`,
                [ text("state", state_lines(k, head, count)) ], "return");
            leave(out[out.len() - 1]);

        } else if name == "isFull" {
            enter("isFull");
            out.push(if count == k { "true" } else { "false" });
            step("isfull", `count == k is ${count == k}.`,
                [ text("state", state_lines(k, head, count)) ], "return");
            leave(out[out.len() - 1]);
        }
    }

    let answer = join(out, " ");
    log(answer, "result");
    result(answer);
}
