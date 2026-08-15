// Solving Questions With Brainpower — backwards, because the cost lands ahead.
//
// Taking question i forbids a block of questions after it. Scanning forwards,
// that block has not been decided yet, so "take" cannot be priced. Scanning
// backwards it has: dp[i + brainpower + 1] is already final, so taking is
// worth points plus that, and skipping is worth dp[i+1]. The brainpower
// becomes a jump length rather than a constraint to track.

fn validate(input) {
    if input.points.len() != input.brainpower.len() {
        return "points and brainpower must have the same length";
    }
    if input.points.len() == 0 {
        return "there must be at least one question";
    }
}

fn labels(points, brain) {
    let out = [];
    for i in 0..points.len() { out.push(`${points[i]}p/skip${brain[i]}`); }
    out
}

fn trace(input) {
    let points = input.points;
    let brain = input.brainpower;
    let n = points.len();

    enter("mostPoints", #{ n: N(n) });

    let dp = [];
    for i in 0..(n + 1) { dp.push(0); }
    step("init", "dp[i] is the most that can be earned starting from question i. dp[n] = 0 — past the end there is nothing left to earn.",
        [ array("questions", labels(points, brain)), array("dp", dp) ]);

    let i = n - 1;
    while i >= 0 {
        step("loop", `Question ${i}. Everything after it is already solved, which is the point of going backwards.`,
            [ array("questions", labels(points, brain)).ptr("i", i).done(after(i, n)),
              array("dp", dp) ]);

        step("read", `It is worth ${points[i]} points and costs ${brain[i]} brainpower.`,
            [ array("questions", labels(points, brain)).ptr("i", i).hl([i]) ]);

        let next = i + brain[i] + 1;
        step("jump", `Taking it means skipping the next ${plural(brain[i], "question")}, so the next available question is ${next}.`,
            [ array("questions", labels(points, brain)).ptr("i", i)
                .bad(span(i + 1, if next - 1 < n - 1 { next - 1 } else { n - 1 })) ]);

        let take = points[i];
        step("take", `Taking it earns ${points[i]} straight away.`,
            [ text("options", [`take = ${take} so far`]) ]);

        if next <= n {
            step("inRange", `Question ${next} exists (or is exactly the end), so the rest of the run is already priced at dp[${next}] = ${dp[next]}.`,
                [ array("dp", dp).ptr("next", next).hl([next]) ]);
            take += dp[next];
            step("addFuture", `Total for taking: ${points[i]} + ${dp[next]} = ${take}.`,
                [ text("options", [`take = ${take}`]) ]);
        } else {
            step("inRange", `The jump lands past the end, so taking this question finishes the run — nothing to add.`,
                [ array("dp", dp).ptr("i", i) ]);
        }

        let skip = dp[i + 1];
        step("skip", `Skipping it leaves question ${i + 1} available, worth dp[${i + 1}] = ${skip}.`,
            [ array("dp", dp).ptr("skip", i + 1).hl([i + 1]),
              text("options", [`take = ${take}`, `skip = ${skip}`]) ]);

        if take > skip {
            step("compare", `${take} beats ${skip}, so solving question ${i} is worth the brainpower.`,
                [ text("options", [`take = ${take}`, `skip = ${skip}`]) ]);
            dp[i] = take;
            log(`dp[${i}] = ${take} (solve it)`);
            step("storeTake", `dp[${i}] = ${dp[i]}.`,
                [ array("dp", dp).ptr("i", i).hl([i]) ]);
        } else {
            step("compare", `${skip} is at least ${take}, so the points are not worth what they cost.`,
                [ text("options", [`take = ${take}`, `skip = ${skip}`]) ]);
            dp[i] = skip;
            log(`dp[${i}] = ${skip} (skip it)`);
            step("storeSkip", `dp[${i}] = ${dp[i]}.`,
                [ array("dp", dp).ptr("i", i).hl([i]) ]);
        }

        i -= 1;
    }

    log(`answer ${dp[0]}`, "result");
    result(`${dp[0]}`);
    step("ret", `Starting from question 0, the most obtainable is ${dp[0]}.`,
        [ bars("dp", dp).ptr("start", 0).hl([0]) ], "return");
    leave(`${dp[0]}`);
}
