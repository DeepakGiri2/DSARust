// Matchsticks to Square — three prunings, and the third is the subtle one.
//
// The arithmetic checks are obvious. Sorting longest-first is a heuristic with
// teeth: a long stick that does not fit anywhere fails at depth 1 rather than
// after the whole tree beneath it has been explored.
//
// The symmetry rule is the interesting one. If two sides currently have the
// same length, putting this stick on either produces the same problem with the
// side labels permuted — so trying the second one is pure repetition. Skipping
// it prunes a large factor without ever losing a solution.

fn sides_view(sides, side) {
    let out = [];
    for s in 0..4 { out.push(`side ${s}: ${sides[s]} of ${side}`); }
    out
}

fn back(sticks, i, sides, side) {
    enter(`back(stick ${i})`, #{ i: N(i) });

    if i == sticks.len() {
        step("placedAll", "Every matchstick has been placed, and no side ever exceeded its length.",
            [ array("matchsticks (longest first)", sticks).done(seq(0, sticks.len() - 1)),
              bars("side lengths", sides) ]);
        step("yes", "So the square can be built.", [ bars("side lengths", sides) ], "return");
        leave("true");
        return [sides, true];
    }

    for s in 0..4 {
        step("loop", `Stick ${i} is ${sticks[i]} long; try putting it on side ${s}.`,
            [ array("matchsticks (longest first)", sticks).ptr("i", i).done(before(i)),
              bars("side lengths", sides),
              text("sides", sides_view(sides, side)) ]);

        if sides[s] + sticks[i] > side {
            step("overflow", `Side ${s} holds ${sides[s]}; adding ${sticks[i]} would make ${sides[s] + sticks[i]}, past ${side}.`,
                [ bars("side lengths", sides).bad([s]) ]);
            continue;
        }

        if s > 0 && sides[s] == sides[s - 1] {
            step("symmetric", `Side ${s} is currently ${sides[s]} long, the same as side ${s - 1}. Putting the stick here gives exactly the problem already explored with the labels swapped — so skip it.`,
                [ bars("side lengths", sides).bad([s]).hl([s - 1]) ]);
            continue;
        }

        sides[s] += sticks[i];
        step("place", `Put it on side ${s}, which is now ${sides[s]} of ${side}.`,
            [ bars("side lengths", sides).hl([s]),
              text("sides", sides_view(sides, side)) ]);

        step("recurse", `Place stick ${i + 1}.`,
            [ array("matchsticks (longest first)", sticks).ptr("i", i + 1).done(before(i + 1)) ],
            "call");
        let out = back(sticks, i + 1, sides, side);
        sides = out[0];

        if out[1] {
            step("propagate", "That branch succeeded, so there is nothing left to try.",
                [ bars("side lengths", sides) ], "return");
            leave("true");
            return [sides, true];
        }

        sides[s] -= sticks[i];
        step("undo", `That did not work out. Take the stick back off side ${s}.`,
            [ bars("side lengths", sides),
              text("sides", sides_view(sides, side)) ]);
    }

    step("no", `No side can take stick ${i}.`, [ bars("side lengths", sides) ], "return");
    leave("false");
    [sides, false]
}

fn trace(input) {
    let sticks = input.matchsticks;
    let n = sticks.len();

    enter("makesquare", #{ n: N(n) });

    let total = 0;
    for m in sticks { total += m; }
    set(#{ total: N(total) });
    step("sum", `The matchsticks total ${total}.`, [ array("matchsticks", sticks) ]);

    if total % 4 != 0 {
        log(`${total} is not divisible by 4`, "result");
        step("divisible", `${total} does not divide by 4, so four equal sides are impossible before any search begins.`,
            [ array("matchsticks", sticks), text("perimeter", [`total: ${total}`]) ], "return");
        result("false");
        leave("false");
        return;
    }
    step("divisible", `${total} divides by 4.`, [ array("matchsticks", sticks) ]);

    let side = total / 4;
    set(#{ side: N(side) });
    step("side", `Each side must be exactly ${side}.`,
        [ text("perimeter", [`total: ${total}`, `side: ${side}`]) ]);

    for i in 1..n {
        let j = i;
        while j > 0 && sticks[j - 1] < sticks[j] {
            swap_at(sticks, j - 1, j);
            j -= 1;
        }
    }
    step("sort", "Longest first. A stick that cannot be placed then fails near the top of the search tree instead of after everything below it has been tried.",
        [ array("matchsticks (longest first)", sticks) ]);

    if sticks[0] > side {
        log(`longest stick ${sticks[0]} exceeds side ${side}`, "result");
        step("tooLong", `The longest stick is ${sticks[0]}, more than a whole side of ${side} — and sticks cannot be broken.`,
            [ array("matchsticks (longest first)", sticks).bad([0]) ], "return");
        result("false");
        leave("false");
        return;
    }
    step("tooLong", `The longest stick, ${sticks[0]}, fits within a side of ${side}.`,
        [ array("matchsticks (longest first)", sticks).hl([0]) ]);

    let sides = [0, 0, 0, 0];
    step("sides", "Four sides, all empty.", [ bars("side lengths", sides) ]);

    step("call", "Place the sticks one at a time.",
        [ array("matchsticks (longest first)", sticks) ], "call");
    let out = back(sticks, 0, sides, side);
    let ok = out[1];

    log(`answer ${ok}`, "result");
    result(`${ok}`);
    leave(`${ok}`);
}
