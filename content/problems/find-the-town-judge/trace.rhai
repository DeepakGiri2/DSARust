fn parse_pairs(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

// Find the Town Judge — one number instead of two degrees.
//
// The judge is characterised by degrees alone: trusted by all n-1 others,
// trusting nobody. Keeping in-degree and out-degree separately works, but a
// single score does the same job — because the maximum reachable score is n-1,
// and only someone who never trusts anyone can reach it. One subtraction
// permanently disqualifies a person.

fn node_labels(n) {
    let out = [];
    for i in 1..(n + 1) { out.push(`${i}`); }
    out
}

fn edges_of(pairs) {
    let out = [];
    for p in pairs { out.push([p[0] - 1, p[1] - 1, true]); }
    out
}

fn scores_view(score, n) {
    let out = [];
    for i in 1..(n + 1) { out.push(score[i]); }
    out
}

fn trace(input) {
    let n = input.n;
    let pairs = parse_pairs(input.trust);
    let nodes = node_labels(n);
    let edges = edges_of(pairs);

    enter("findJudge", #{ n: N(n) });

    let score = [];
    for i in 0..(n + 1) { score.push(0); }
    step("init", `One score per person. A judge would need ${n - 1} — trusted by everyone else, trusting nobody.`,
        [ graph("who trusts whom", nodes, edges),
          array("score per person (1..n)", scores_view(score, n)) ]);

    for k in 0..pairs.len() {
        let a = pairs[k][0];
        let b = pairs[k][1];
        step("loop", `Edge ${k}: ${a} trusts ${b}.`,
            [ graph("who trusts whom", nodes, edges).active([[a - 1, b - 1]]),
              array("score per person (1..n)", scores_view(score, n)) ]);

        score[a] -= 1;
        step("truster", `${a} trusts someone, so ${a} cannot be the judge — ever. Score ${score[a]}.`,
            [ graph("who trusts whom", nodes, edges).bad([a - 1]),
              array("score per person (1..n)", scores_view(score, n)).bad([a - 1]) ]);

        score[b] += 1;
        step("trusted", `${b} gains a truster: score ${score[b]}.`,
            [ graph("who trusts whom", nodes, edges).frontier([b - 1]),
              array("score per person (1..n)", scores_view(score, n)).hl([b - 1]) ]);
    }

    let judge = -1;
    for i in 1..(n + 1) {
        step("scan", `Person ${i} has score ${score[i]}.`,
            [ graph("who trusts whom", nodes, edges).cur(i - 1),
              array("score per person (1..n)", scores_view(score, n)).ptr("i", i - 1) ]);

        if score[i] == n - 1 {
            judge = i;
            log(`judge is ${i}`, "result");
            step("check", `${n - 1} is the maximum possible, and only someone who never trusts anyone can reach it.`,
                [ graph("who trusts whom", nodes, edges).done([i - 1]),
                  array("score per person (1..n)", scores_view(score, n)).hl([i - 1]) ]);
            step("ret", `Person ${i} is the judge.`,
                [ graph("who trusts whom", nodes, edges).done([i - 1]) ], "return");
            break;
        }

        step("check", `${score[i]} is not ${n - 1}, so person ${i} is not the judge.`,
            [ array("score per person (1..n)", scores_view(score, n)).bad([i - 1]) ]);
    }

    if judge < 0 {
        log("no judge", "result");
        step("none", "Nobody reached the required score, so there is no judge.",
            [ graph("who trusts whom", nodes, edges),
              array("score per person (1..n)", scores_view(score, n)) ], "return");
    }

    result(`${judge}`);
    leave(`${judge}`);
}
