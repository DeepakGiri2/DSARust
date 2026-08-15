// Factorial Trailing Zeroes — count the fives, never the factorial.
//
// 21! already overflows a 64-bit integer, so any solution that multiplies is
// wrong before it starts. Each trailing zero is a 2×5 pair, and twos are much
// more common than fives in 1·2·3·…·n, so the fives are the bottleneck.
//
// Why the powers: 25 = 5×5 contributes two fives, but it was already counted
// once by the n/5 term, so n/25 adds its *second* five. n/125 adds a third,
// and so on — each term corrects the one before it rather than restating it.

fn trace(input) {
    let n = input.n;

    enter("trailingZeroes", #{ n: N(n) });

    let count = 0;
    set(#{ count: N(0) });
    step("init", `Counting the factors of 5 in ${n}! — never computing ${n}! itself, which would overflow long before ${n}.`,
        [ text("working", [`n = ${n}`, "count = 0"]) ]);

    let p = 5;
    let lines = [];
    while p <= n {
        set(#{ p: N(p) });
        step("loop", `p = ${p}: how many numbers up to ${n} are divisible by ${p}?`,
            [ text("working", [`n = ${n}`, `p = ${p}`, `count = ${count}`]) ]);

        let add = n / p;
        count += add;
        lines.push(`${n} / ${p} = ${add}`);
        set(#{ count: N(count) });
        step("add",
            `${add}. ${if p == 5 { "That is one five each." } else { `Each of those already gave a five to an earlier term, so this counts their extra one.` }} count = ${count}.`,
            [ text("working", lines), text("total", [`count = ${count}`]) ]);

        p *= 5;
    }

    if lines.is_empty() {
        step("loop", `5 is already bigger than ${n}, so ${n}! has no factor of five at all.`,
            [ text("working", [`n = ${n}`]) ]);
    }

    log(`${count} trailing zeroes`, "result");
    result(`${count}`);
    step("ret", `${n}! ends in ${plural(count, "zero")}, found in ${plural(lines.len(), "division")}.`,
        [ text("working", lines), text("total", [`count = ${count}`]) ], "return");
    leave(`${count}`);
}
