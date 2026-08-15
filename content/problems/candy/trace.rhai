// Candy — one constraint per direction, one pass each.
//
// "More than either neighbour" cannot be enforced in a single sweep, because
// the right-hand neighbour has not been settled yet. Splitting the rule fixes
// that: sweeping right satisfies every left-hand constraint, and sweeping left
// then satisfies the right-hand ones. The second pass raises a value only when
// it is genuinely too small, which is why the first pass's guarantees are not
// destroyed — and why nothing ends up larger than it has to be.

fn trace(input) {
    let ratings = input.ratings;
    let n = ratings.len();

    enter("candy", #{ n: N(n) });

    let candies = [];
    for i in 0..n { candies.push(0); }
    step("init", "One count per child.",
        [ bars("ratings", ratings), array("sweets", candies) ]);

    for i in 0..n { candies[i] = 1; }
    step("seed", "Everybody gets at least one — that alone satisfies the rule wherever two neighbours are rated equally.",
        [ bars("ratings", ratings), array("sweets", candies) ]);

    for i in 1..n {
        step("leftPass", `Left to right, child ${i}.`,
            [ bars("ratings", ratings).ptr("i", i).done(before(i)),
              array("sweets", candies) ]);

        if ratings[i] > ratings[i - 1] {
            step("higherLeft", `Rated ${ratings[i]} against ${ratings[i - 1]} on the left, so this child must get more than that neighbour.`,
                [ bars("ratings", ratings).ptr("i", i).hl([i - 1, i]),
                  array("sweets", candies).hl([i - 1]) ]);
            candies[i] = candies[i - 1] + 1;
            log(`child ${i} needs ${candies[i]} (left rule)`);
            step("bumpLeft", `Exactly one more than the left neighbour: ${candies[i]}. One more than necessary would only inflate the total.`,
                [ array("sweets", candies).hl([i]) ]);
        } else {
            step("higherLeft", `Rated ${ratings[i]} against ${ratings[i - 1]} — no left-hand constraint applies.`,
                [ bars("ratings", ratings).ptr("i", i).hl([i - 1, i]) ]);
        }
    }

    let i = n - 2;
    while i >= 0 {
        step("rightPass", `Right to left, child ${i}. The left-hand rule is already satisfied everywhere, so this pass must not break it.`,
            [ bars("ratings", ratings).ptr("i", i).done(after(i, n)),
              array("sweets", candies) ]);

        if ratings[i] > ratings[i + 1] && candies[i] <= candies[i + 1] {
            step("higherRight", `Rated ${ratings[i]} against ${ratings[i + 1]} on the right, but holding only ${candies[i]} against ${candies[i + 1]} — that violates the rule.`,
                [ bars("ratings", ratings).ptr("i", i).hl([i, i + 1]),
                  array("sweets", candies).bad([i]) ]);
            candies[i] = candies[i + 1] + 1;
            log(`child ${i} raised to ${candies[i]} (right rule)`);
            step("bumpRight", `Raise it to ${candies[i]}. Raising can never break a left-hand constraint, which is why the two passes compose.`,
                [ array("sweets", candies).hl([i]) ]);
        } else {
            step("higherRight",
                if ratings[i] > ratings[i + 1] {
                    `Rated higher than the right neighbour, but already holding ${candies[i]} against ${candies[i + 1]} — the left pass covered it.`
                } else {
                    `Not rated above the right neighbour, so no constraint applies.`
                },
                [ bars("ratings", ratings).ptr("i", i).hl([i, i + 1]),
                  array("sweets", candies) ]);
        }
        i -= 1;
    }

    let total = 0;
    set(#{ total: N(total) });
    step("sum", "Add it all up.", [ array("sweets", candies) ]);

    for k in 0..n {
        total += candies[k];
        set(#{ total: N(total) });
        step("add", `+ ${candies[k]} = ${total}.`,
            [ array("sweets", candies).ptr("k", k).done(before(k)) ]);
    }

    log(`answer ${total}`, "result");
    result(`${total}`);
    step("ret", `${total} sweets, and no distribution satisfying both rules can use fewer.`,
        [ bars("ratings", ratings), array("sweets", candies) ], "return");
    leave(`${total}`);
}
