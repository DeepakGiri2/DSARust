// Climbing Stairs — memoised recursion.
//
// Two things to see here. The call stack panel shows the tree of calls being
// explored depth-first; the memo table shows why it does not explode — the
// second time a value is asked for, the recursion stops dead at the lookup.

fn climb(memo, n, calls) {
    enter(`climb(${n})`, #{ n: N(n) });

    if n <= 2 {
        step("base", `n = ${n} is a base case: there is exactly ${n} way(s) to climb ${n} step(s).`,
            [ kv("memo — n → ways", memo) ]);
        log(`climb(${n}) = ${n} (base case)`);
        step("retbase", `Return ${n} without recursing further.`,
            [ kv("memo — n → ways", memo) ], "return");
        leave(`${n}`);
        return [memo, n, calls + 1];
    }

    if map_has(memo, n) {
        let v = map_get(memo, n);
        step("memo", `climb(${n}) has been computed before — the memo already holds it.`,
            [ kv("memo — n → ways", memo).hl([n]) ]);
        log(`memo hit: climb(${n}) = ${v}`, "result");
        step("retmemo",
            `Return ${v} straight from the table. Without this line the call tree would double in size at every level.`,
            [ kv("memo — n → ways", memo).hl([n]) ], "return");
        leave(`${v}`);
        return [memo, v, calls + 1];
    }

    step("rec",
        `To reach step ${n} you arrive either from step ${n - 1} (one step) or step ${n - 2} (two steps), so climb(${n}) = climb(${n - 1}) + climb(${n - 2}).`,
        [ kv("memo — n → ways", memo) ], "call");

    let a = climb(memo, n - 1, calls + 1);
    memo = a[0];
    let left = a[1];
    calls = a[2];

    let b = climb(memo, n - 2, calls);
    memo = b[0];
    let right = b[1];
    calls = b[2];

    let total = left + right;
    map_set(memo, n, total);
    set(#{ result: N(total) });
    log(`store memo[${n}] = ${left} + ${right} = ${total}`);
    step("store", `climb(${n - 1}) = ${left} and climb(${n - 2}) = ${right}, so climb(${n}) = ${total}. File it in the memo.`,
        [ kv("memo — n → ways", memo).hl([n]) ]);

    step("ret", `Return ${total} to the caller.`, [ kv("memo — n → ways", memo).hl([n]) ], "return");
    leave(`${total}`);
    [memo, total, calls]
}

fn trace(input) {
    let n = input.n;
    let memo = map_new();

    enter("climbStairs", #{ n: N(n) });
    step("init", "The memo starts empty. Every value it gains is a subproblem that will never be recomputed.",
        [ kv("memo — n → ways", memo) ]);
    step("call", `Kick off the recursion with climb(${n}).`, [ kv("memo — n → ways", memo) ], "call");

    let out = climb(memo, n, 0);
    memo = out[0];
    let answer = out[1];
    let calls = out[2];

    log(`${calls} call(s) total, ${map_len(memo)} value(s) memoised`, "result");
    result(`${answer}`);
    step("call",
        `${answer} distinct ways to climb ${n} steps — reached in ${calls} calls instead of the exponential number plain recursion would need.`,
        [ kv("memo — n → ways", memo) ], "return");
    leave(`${answer}`);
}
