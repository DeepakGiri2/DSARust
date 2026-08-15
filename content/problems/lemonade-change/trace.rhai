// Lemonade Change — a $5 is worth more than its face value.
//
// The only genuine choice is how to make $15 of change: one $10 and one $5, or
// three $5s. Prefer the $10 every time. The reason is that a $10 note can only
// ever be given as change for a $20, while a $5 is needed for both a $10 and a
// $20 — so a $5 is strictly the more flexible asset, and spending three of
// them to save a note that is less useful is always a bad trade.

fn trace(input) {
    let bills = input.bills;
    let n = bills.len();

    enter("lemonadeChange", #{});

    let five = 0;
    let ten = 0;
    set(#{ five: N(five), ten: N(ten) });
    step("init", "The till starts empty; change can only come from earlier customers.",
        [ array("bills, in order", bills), text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);

    let ok = true;
    for i in 0..n {
        let b = bills[i];
        step("loop", `Customer ${i} pays with a $${b}.`,
            [ array("bills, in order", bills).ptr("i", i).done(before(i)),
              text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);

        if b == 5 {
            step("fiver", "Exactly the price, so no change is needed.",
                [ array("bills, in order", bills).ptr("i", i).hl([i]) ]);
            five += 1;
            set(#{ five: N(five) });
            step("keepFive", `The till gains a $5, which is the most useful note there is: ${five} of them.`,
                [ text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);
        } else if b == 10 {
            step("tenner", "A $10 needs $5 back, and only a $5 note will do — there is no choice here.",
                [ array("bills, in order", bills).ptr("i", i).hl([i]),
                  text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);
            if five == 0 {
                ok = false;
                log("no $5 for change", "result");
                step("noFive", "There is no $5 in the till, so this customer cannot be served.",
                    [ array("bills, in order", bills).bad([i]),
                      text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ], "return");
                break;
            }
            five -= 1;
            set(#{ five: N(five) });
            step("giveFive", `Give a $5 away: ${five} left.`,
                [ text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);
            ten += 1;
            set(#{ ten: N(ten) });
            step("keepTen", `And take the $10: ${ten} of them.`,
                [ text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);
        } else {
            step("twenty", "A $20 needs $15 back — the only place a real decision arises.",
                [ array("bills, in order", bills).ptr("i", i).hl([i]),
                  text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);

            if ten > 0 && five > 0 {
                step("preferTen", "A $10 plus a $5 is available. Prefer it: a $10 can only ever be change for a $20, whereas a $5 is needed for both — so keeping $5s is always at least as good.",
                    [ text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);
                ten -= 1;
                five -= 1;
                set(#{ five: N(five), ten: N(ten) });
                step("giveTenFive", `Till is now ${five} × $5 and ${ten} × $10.`,
                    [ text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);
            } else if five >= 3 {
                step("threeFives", "No $10 available, so three $5s it has to be.",
                    [ text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);
                five -= 3;
                set(#{ five: N(five) });
                step("giveThree", `Till is now ${five} × $5 and ${ten} × $10.`,
                    [ text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);
            } else {
                ok = false;
                log("cannot make $15", "result");
                step("stuck", `${five} × $5 and ${ten} × $10 cannot make $15.`,
                    [ array("bills, in order", bills).bad([i]),
                      text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ]);
                step("no", "So this customer cannot be served.",
                    [ text("answer", ["false"]) ], "return");
                break;
            }
        }
    }

    if ok {
        log("everyone served", "result");
        step("yes", "Every customer received exact change.",
            [ text("till", [`$5 notes: ${five}`, `$10 notes: ${ten}`]) ], "return");
    }

    result(`${ok}`);
    leave(`${ok}`);
}
