// Happy Number — the sequence is a linked list, so cycle detection applies.
//
// Each number has exactly one successor, which makes the sequence a chain.
// An unhappy number therefore does not run forever into new values — it must
// loop, and a tortoise and hare finds that loop without remembering anything.

fn next_of(n) {
    let sum = 0;
    while n > 0 {
        let d = n % 10;
        sum += d * d;
        n = n / 10;
    }
    sum
}

fn digits_note(n) {
    let parts = [];
    let m = n;
    while m > 0 {
        let d = m % 10;
        parts.push(`${d}²=${d * d}`);
        m = m / 10;
    }
    join(parts, " + ")
}

fn trace(input) {
    let n = input.n;
    let slow = n;
    let fast = next_of(n);
    let seen = [n, fast];

    enter("isHappy", #{ n: N(n) });
    set(#{ slow: N(slow), fast: N(fast) });
    step("init", `slow starts at ${n}; fast starts one step ahead at ${fast} (${digits_note(n)}).`,
        [ array("sequence", seen).pointers(#{ slow: 0, fast: 1 }) ]);

    let guard = 0;
    while fast != 1 && slow != fast && guard < 60 {
        guard += 1;
        step("loop", `fast is at ${fast}, slow at ${slow} — no 1 and no meeting yet.`,
            [ array("sequence", seen) ]);

        slow = next_of(slow);
        set(#{ slow: N(slow) });
        step("slow", `slow advances one: ${digits_note(slow)} … now ${slow}.`,
            [ array("sequence", seen) ]);

        fast = next_of(next_of(fast));
        if !seen.contains(fast) { seen.push(fast); }
        set(#{ fast: N(fast) });
        log(`slow=${slow} fast=${fast}`);
        step("fast", `fast advances two: now ${fast}.`,
            [ array("sequence", seen).hl([seen.index_of(fast)]) ]);
    }

    let happy = fast == 1;
    log(`happy: ${happy}`, "result");
    result(`${happy}`);
    step("ret",
        if happy {
            "fast reached 1, so the sequence terminates — the number is happy.".to_string()
        } else {
            `slow and fast met at ${fast} without reaching 1, so the sequence is stuck in a cycle.`
        },
        [ array("sequence", seen).hl(if happy { [] } else { [seen.index_of(fast)] }) ],
        "return");
    leave(`${happy}`);
}
