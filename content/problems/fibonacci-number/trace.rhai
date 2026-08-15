// Fibonacci — the whole table collapses to two variables.
//
// The recursive definition recomputes F(n-2) inside both F(n-1) and F(n-2),
// which is where the exponential blow-up comes from. Computing forwards visits
// each term once. And because a term reaches back exactly two places, there is
// never a reason to keep the rest of the table — two variables is the entire
// state.

fn trace(input) {
    let n = input.n;

    enter("fib", #{ n: N(n) });

    if n < 2 {
        step("base", `F(${n}) = ${n} by definition — the recursion has to stop somewhere.`,
            [ text("definition", ["F(0) = 0", "F(1) = 1", "F(k) = F(k-1) + F(k-2)"]) ]);
        log(`answer ${n}`, "result");
        result(`${n}`);
        step("ret", `Return ${n}.`, [ text("answer", [`F(${n}) = ${n}`]) ], "return");
        leave(`${n}`);
        return;
    }
    step("base", `${n} is past the base cases, so the terms have to be built up.`,
        [ text("definition", ["F(0) = 0", "F(1) = 1", "F(k) = F(k-1) + F(k-2)"]) ]);

    let prev = 0;
    let cur = 1;
    let seq = [0, 1];
    set(#{ prev: N(prev), cur: N(cur) });
    step("init", "Start from the two base cases. Everything else follows from them.",
        [ array("terms computed", seq).hl([0, 1]),
          text("state", [`prev = F(0) = ${prev}`, `cur = F(1) = ${cur}`]) ]);

    for i in 2..(n + 1) {
        step("loop", `Building F(${i}).`,
            [ array("terms computed", seq).hl([seq.len() - 2, seq.len() - 1]),
              text("state", [`prev = F(${i - 2}) = ${prev}`, `cur = F(${i - 1}) = ${cur}`]) ]);

        let next = prev + cur;
        log(`F(${i}) = ${prev} + ${cur} = ${next}`);
        step("add", `F(${i}) = F(${i - 2}) + F(${i - 1}) = ${prev} + ${cur} = ${next}.`,
            [ array("terms computed", seq).hl([seq.len() - 2, seq.len() - 1]),
              text("state", [`next = ${next}`]) ]);

        prev = cur;
        set(#{ prev: N(prev) });
        step("shift", `F(${i - 1}) = ${prev} becomes the older of the two.`,
            [ text("state", [`prev = F(${i - 1}) = ${prev}`]) ]);

        cur = next;
        seq.push(cur);
        set(#{ cur: N(cur) });
        step("advance", `F(${i}) = ${cur} becomes the newer. F(${i - 2}) is now unreachable, and never needed again.`,
            [ bars("terms computed", seq).hl([seq.len() - 1]),
              text("state", [`prev = F(${i - 1}) = ${prev}`, `cur = F(${i}) = ${cur}`]) ]);
    }

    log(`answer ${cur}`, "result");
    result(`${cur}`);
    step("ret", `F(${n}) = ${cur}, computed in ${n - 1} addition(s) and two variables' worth of memory.`,
        [ bars("terms computed", seq).hl([seq.len() - 1]) ], "return");
    leave(`${cur}`);
}
