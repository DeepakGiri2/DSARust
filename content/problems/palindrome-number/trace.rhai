// Palindrome Number — reverse only half of it.
//
// Reversing the whole number would work but can overflow, and it does twice
// the necessary work. Peeling digits off the back until the reversed part
// catches up leaves exactly the front half and the back half facing each
// other. When the digit count is odd, the middle digit ends up on the reversed
// side and is dropped by a single division — which is the whole reason for the
// second half of the final comparison.

fn trace(input) {
    let original = input.x;
    let x = original;

    enter("isPalindrome", #{ x: N(x) });

    if x < 0 || (x % 10 == 0 && x != 0) {
        let why = if x < 0 {
            `${x} is negative, and the minus sign has no mirror at the other end.`
        } else {
            `${x} ends in 0, so a palindrome would have to start with 0 — which no integer does.`
        };
        log(`rejected: ${why}`, "result");
        step("reject", why, [ text("number", [`x = ${x}`]) ], "return");
        result("false");
        leave("false");
        return;
    }
    step("reject", `${x} is neither negative nor a multiple of 10, so it could still be a palindrome.`,
        [ text("number", [`x = ${x}`]) ]);

    let rev = 0;
    set(#{ rev: N(rev) });
    step("init", "Build the reversed back half here.",
        [ text("state", [`front (x) = ${x}`, `reversed back (rev) = ${rev}`]) ]);

    while x > rev {
        step("loop", `${x} is still bigger than ${rev}, so fewer than half the digits have moved across.`,
            [ text("state", [`front (x) = ${x}`, `reversed back (rev) = ${rev}`]) ]);

        let d = x % 10;
        rev = rev * 10 + d;
        set(#{ rev: N(rev) });
        step("take", `Take the last digit, ${d}, and push it onto the reversed value: ${rev}.`,
            [ text("state", [`last digit taken: ${d}`, `reversed back = ${rev}`]) ]);

        x /= 10;
        set(#{ x: N(x) });
        step("drop", `Drop it from the front half: ${x}.`,
            [ text("state", [`front (x) = ${x}`, `reversed back (rev) = ${rev}`]) ]);
    }

    let even_ok = x == rev;
    let odd_ok = x == rev / 10;
    let ok = even_ok || odd_ok;
    log(`answer ${ok}`, "result");
    result(`${ok}`);
    step("ret",
        if even_ok {
            `The two halves are both ${x} — an even number of digits, and they match.`
        } else if odd_ok {
            `An odd number of digits, so ${rev} carries the middle one. Dropping it gives ${rev / 10}, which matches the front half ${x}.`
        } else {
            `The front half is ${x} and the reversed back half is ${rev} — they do not match, with or without a middle digit.`
        },
        [ text("answer", [`${original} is ${if ok { "" } else { "not " }}a palindrome`]) ], "return");
    leave(`${ok}`);
}
