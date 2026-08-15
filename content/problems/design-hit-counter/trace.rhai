// Design Hit Counter — the window slides, so only the front can expire.
//
// Timestamps arrive in order, which is the whole reason a queue works: the
// oldest hit is always at the front, so "which hits have expired?" is answered
// by dropping from one end until the first survivor. Each timestamp enters and
// leaves at most once, so although a single getHits may drop many, the cost
// per hit averages out to constant.
//
// The follow-up question interviewers ask next — "what if there are thousands
// of hits per second?" — is answered by bucketing: 300 slots of
// (timestamp, count), overwritten when a slot goes stale. Same idea, bounded
// memory.

const WINDOW = 300;

fn parse_ops(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if !w.is_empty() { out.push(w); }
    }
    out
}

fn trace(input) {
    let ops = parse_ops(input.ops);

    enter("HitCounter");
    let q = [];
    step("init", "One queue of timestamps. Nothing else is stored — the size *is* the answer.",
        [ queue("timestamps", q) ]);
    leave("ready");

    let out = [];
    for o in ops {
        let t = num(o[1]);

        if o[0] == "hit" {
            enter(`hit(${t})`, #{ timestamp: N(t) });
            step("hit", `A hit at t = ${t}.`, [ queue("timestamps", q) ]);
            q.push(t);
            step("push", `Push it on the back. Timestamps never decrease, so the queue stays sorted for free.`,
                [ queue("timestamps", q).pushed(true).hl([q.len() - 1]) ], "return");
            leave("");
            continue;
        }

        enter(`getHits(${t})`, #{ timestamp: N(t) });
        let cutoff = t - global::WINDOW;
        set(#{ cutoff: N(cutoff) });
        step("get", `getHits(${t}) — count the hits in (${cutoff}, ${t}]. Anything at ${cutoff} or earlier has expired.`,
            [ queue("timestamps", q), text("window", [`now    = ${t}`, `cutoff = ${cutoff}`]) ]);

        let dropped = 0;
        while !q.is_empty() && q[0] <= cutoff {
            step("stale", `The front timestamp ${q[0]} is ≤ ${cutoff} — outside the window.`,
                [ queue("timestamps", q).bad([0]),
                  text("window", [`now    = ${t}`, `cutoff = ${cutoff}`]) ]);
            let gone = q[0];
            q.remove(0);
            dropped += 1;
            step("drop", `Drop ${gone}. Only the front can expire, so one look is enough to know whether to keep going.`,
                [ queue("timestamps", q).popped(true) ]);
        }
        if dropped == 0 {
            step("stale", "The front is already inside the window, so nothing expires.",
                [ queue("timestamps", q), text("window", [`now    = ${t}`, `cutoff = ${cutoff}`]) ]);
        }

        out.push(`${q.len()}`);
        log(`getHits(${t}) = ${q.len()}`, "result");
        step("count", `${plural(q.len(), "hit")} remain in the window — that is the answer, no counting loop required.`,
            [ queue("timestamps", q).hl(seq(0, q.len() - 1)),
              text("window", [`now    = ${t}`, `answer = ${q.len()}`]) ], "return");
        leave(`${q.len()}`);
    }

    let answer = join(out, " ");
    log(answer, "result");
    result(answer);
}
