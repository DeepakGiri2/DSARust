// Plus One — carry propagation, stopped as early as possible.
//
// The loop exists only to handle carries. The moment a digit below 9 is
// reached, incrementing it absorbs the carry and nothing further left can
// change — so the function returns from inside the loop rather than
// continuing.

fn trace(input) {
    let digits = input.digits;
    let n = digits.len();

    enter("plusOne", #{ digits: A(digits) });

    let i = n - 1;
    while i >= 0 {
        set(#{ i: N(i) });
        step("loop", `Digit ${i} is ${digits[i]}.`,
            [ array("digits", digits).ptr("i", i).done(after(i, n)) ]);

        if digits[i] < 9 {
            step("check", `${digits[i]} is below 9, so adding one cannot carry.`,
                [ array("digits", digits).ptr("i", i).hl([i]).done(after(i, n)) ]);
            digits[i] += 1;
            log(`digit ${i} -> ${digits[i]}`);
            step("inc", `It becomes ${digits[i]}.`,
                [ array("digits", digits).hl([i]) ]);
            let answer = join(digits, " ");
            result(answer);
            step("done", `Nothing to the left can change, so return immediately: ${answer}.`,
                [ array("digits", digits).hl(seq(0, n - 1)) ], "return");
            leave(answer);
            return;
        }

        step("check", `${digits[i]} is a 9, so adding one carries.`,
            [ array("digits", digits).ptr("i", i).bad([i]) ]);
        digits[i] = 0;
        step("zero", "It becomes 0 and the carry moves left.",
            [ array("digits", digits).ptr("i", i).done(after(i - 1, n)) ]);
        i -= 1;
    }

    let out = [1];
    for d in digits { out.push(d); }
    let answer = join(out, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("grow", `Every digit was a 9, so the number needs one more place: ${answer}.`,
        [ array("digits", out).hl([0]) ], "return");
    leave(answer);
}
