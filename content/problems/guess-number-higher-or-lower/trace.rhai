// Guess Number — binary search with no array in sight.
//
// Nothing is being indexed here. All binary search ever needs is a range and a
// test whose answer rules out one side of it, and the oracle is exactly such a
// test. Guessing the middle is what makes each answer worth the most: it is
// the only choice where neither reply can leave more than half the range.

fn validate(input) {
    if input.pick < 1 || input.pick > input.n {
        return `the picked number must be between 1 and ${input.n}`;
    }
}

fn candidates(lo, hi) {
    let out = [];
    let k = lo;
    while k <= hi { out.push(k); k += 1; }
    out
}

fn trace(input) {
    let n = input.n;
    let pick = input.pick;

    enter("guessNumber", #{ n: N(n) });

    let lo = 1;
    let hi = n;
    let guesses = 0;
    set(#{ lo: N(lo), hi: N(hi) });
    step("bounds", `The number is somewhere in 1..${n} — ${n} possibilities and no other information.`,
        [ array("still possible", candidates(lo, hi)),
          text("state", [`range: ${lo}..${hi}`, `possibilities: ${hi - lo + 1}`]) ]);

    let found = 0;
    while lo <= hi {
        step("loop", `${plural(hi - lo + 1, "candidate")} left, from ${lo} to ${hi}.`,
            [ array("still possible", candidates(lo, hi)),
              text("state", [`range: ${lo}..${hi}`, `guesses so far: ${guesses}`]) ]);

        let mid = lo + (hi - lo) / 2;
        set(#{ mid: N(mid) });
        step("mid", `Guess the middle, ${mid}. Any other guess risks leaving more than half the range behind.`,
            [ array("still possible", candidates(lo, hi)).hl([mid - lo]),
              text("state", [`guessing: ${mid}`]) ]);

        guesses += 1;
        let r = if mid == pick { 0 } else if mid > pick { -1 } else { 1 };
        set(#{ guesses: N(guesses) });
        step("ask", `Guess ${guesses}: the oracle says ${if r == 0 { "correct" } else if r < 0 { "too high" } else { "too low" }}.`,
            [ array("still possible", candidates(lo, hi)).hl([mid - lo]),
              text("state", [`guessed ${mid}`,
                             if r == 0 { "answer: correct" } else if r < 0 { "answer: too high" } else { "answer: too low" }]) ]);

        if r == 0 {
            found = mid;
            log(`found ${mid} in ${guesses} guess(es)`, "result");
            step("hit", `${mid} is the number.`,
                [ array("still possible", candidates(lo, hi)).hl([mid - lo]) ]);
            step("ret", `Found in ${plural(guesses, "guess")}, out of ${n} possibilities.`,
                [ text("state", [`answer: ${mid}`, `guesses used: ${guesses}`]) ], "return");
            break;
        }

        if r < 0 {
            step("lower", `Too high, so ${mid} and everything above it are gone — half the range in one question.`,
                [ array("still possible", candidates(lo, hi)).bad(span(mid - lo, hi - lo)) ]);
            hi = mid - 1;
            set(#{ hi: N(hi) });
            step("shrinkHi", `Range is now ${lo}..${hi}.`,
                [ array("still possible", candidates(lo, hi)) ]);
        } else {
            step("higher", `Too low, so ${mid} and everything below it are gone.`,
                [ array("still possible", candidates(lo, hi)).bad(span(0, mid - lo)) ]);
            lo = mid + 1;
            set(#{ lo: N(lo) });
            step("shrinkLo", `Range is now ${lo}..${hi}.`,
                [ array("still possible", candidates(lo, hi)) ]);
        }
    }

    if found == 0 {
        log("no candidate left", "result");
        step("miss", "The range emptied without a match, which can only happen if the oracle is inconsistent.",
            [ text("state", ["no candidates remain"]) ], "return");
        result("-1");
        leave("-1");
        return;
    }

    result(`${found}`);
    leave(`${found}`);
}
