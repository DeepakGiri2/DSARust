fn parse_pairs(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

// Path With Maximum Probability — Dijkstra with the inequality flipped.
//
// Dijkstra needs only one property: extending a path never improves its cost.
// Sums of non-negative weights grow; products of probabilities shrink. Both
// qualify. So the same algorithm works with "smallest unsettled" replaced by
// "largest unsettled" and addition replaced by multiplication — and the
// settled node's value is final for exactly the same reason.

// Parsed by hand rather than with parse_float, so the trace does not depend on
// which string helpers the engine exposes.
fn to_prob(s) {
    let parts = s.split(".");
    let whole = num(parts[0]);
    let out = whole * 1.0;
    if parts.len() > 1 {
        let frac = num(parts[1]);
        let scale = 1.0;
        for i in 0..parts[1].len() { scale = scale * 10.0; }
        out = out + (frac * 1.0) / scale;
    }
    out
}

fn fmt(v) {
    let s = `${v}`;
    if s.ends_with(".0") { s.sub_string(0, s.len() - 2) } else { s }
}

fn node_labels(n) {
    let out = [];
    for i in 0..n { out.push(`${i}`); }
    out
}

fn edge_list(pairs, probs) {
    let out = [];
    for i in 0..pairs.len() { out.push([pairs[i][0], pairs[i][1], false, fmt(probs[i])]); }
    out
}

fn prob_view(prob) {
    let out = [];
    for v in prob { out.push(fmt(v)); }
    out
}

fn settled_ids(visited) {
    let out = [];
    for i in 0..visited.len() { if visited[i] { out.push(i); } }
    out
}

fn trace(input) {
    let n = input.n;
    let pairs = parse_pairs(input.edges);
    let probs = [];
    for s in words(input.succProb) { probs.push(to_prob(s)); }
    let start = input.start;
    let goal = input.end;
    let nodes = node_labels(n);
    let edges = edge_list(pairs, probs);

    enter("maxProbability", #{ start: N(start), end: N(goal) });

    let adj = [];
    for i in 0..n { adj.push([]); }
    for i in 0..pairs.len() {
        adj[pairs[i][0]].push([pairs[i][1], probs[i]]);
        adj[pairs[i][1]].push([pairs[i][0], probs[i]]);
    }
    step("build", `${plural(pairs.len(), "undirected edge")}, each with its own success probability.`,
        [ graph("network", nodes, edges) ]);

    let prob = [];
    for i in 0..n { prob.push(0.0); }
    step("init", "Best known probability of reaching each node.",
        [ array("best probability per node", prob_view(prob)) ]);

    prob[start] = 1.0;
    step("seed", `Being at node ${start} is certain, so its probability is 1.`,
        [ graph("network", nodes, edges).cur(start),
          array("best probability per node", prob_view(prob)).hl([start]) ]);

    let visited = [];
    for i in 0..n { visited.push(false); }
    step("visited", "Settled nodes are final.",
        [ array("best probability per node", prob_view(prob)) ]);

    for iter in 0..n {
        step("loop", `Round ${iter + 1}.`,
            [ graph("network", nodes, edges).done(settled_ids(visited)),
              array("best probability per node", prob_view(prob)) ]);

        let u = -1;
        step("pick", "Which unsettled node has the highest probability?",
            [ array("best probability per node", prob_view(prob)).done(settled_ids(visited)) ]);

        for v in 0..n {
            if !visited[v] && (u == -1 || prob[v] > prob[u]) { u = v; }
        }
        step("scan", "Every unsettled node considered.",
            [ array("best probability per node", prob_view(prob)).done(settled_ids(visited)) ]);

        if prob[u] == 0.0 {
            step("stuck", "The best remaining node is unreachable, so everything still unsettled is too.",
                [ graph("network", nodes, edges).done(settled_ids(visited)) ]);
            break;
        }

        step("highest", `Node ${u} at ${fmt(prob[u])} is the best. Any other route to it would go through a node of lower probability and then multiply by something at most 1 — so it could not do better.`,
            [ graph("network", nodes, edges).cur(u).done(settled_ids(visited)),
              array("best probability per node", prob_view(prob)).hl([u]) ]);

        visited[u] = true;
        log(`settle ${u} at ${fmt(prob[u])}`);
        step("settle", `${fmt(prob[u])} is final for node ${u}.`,
            [ graph("network", nodes, edges).done(settled_ids(visited)) ]);

        for e in adj[u] {
            step("relax", `Edge ${u} – ${e[0]}, succeeding with probability ${fmt(e[1])}.`,
                [ graph("network", nodes, edges).cur(u).active([[u, e[0]]])
                    .done(settled_ids(visited)) ]);

            let p = prob[u] * e[1];
            step("multiply", `Both must succeed: ${fmt(prob[u])} × ${fmt(e[1])} = ${fmt(p)}. Multiplying can only shrink the value, which is the monotonicity Dijkstra needs.`,
                [ text("relaxation", [`route so far: ${fmt(prob[u])}`, `edge: ${fmt(e[1])}`,
                                      `combined: ${fmt(p)}`]) ]);

            if p > prob[e[0]] {
                step("better", `${fmt(p)} beats the ${fmt(prob[e[0]])} known for node ${e[0]}.`,
                    [ array("best probability per node", prob_view(prob)).ptr("to", e[0]) ]);
                prob[e[0]] = p;
                log(`node ${e[0]} reachable with ${fmt(p)}`);
                step("update", `Node ${e[0]} is now ${fmt(p)}.`,
                    [ array("best probability per node", prob_view(prob)).hl([e[0]]) ]);
            } else {
                step("better", `${fmt(p)} does not beat the ${fmt(prob[e[0]])} already known.`,
                    [ array("best probability per node", prob_view(prob)).ptr("to", e[0]) ]);
            }
        }
    }

    let answer = fmt(prob[goal]);
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `The best chance of reaching node ${goal} is ${answer}.`,
        [ graph("network", nodes, edges).done(settled_ids(visited)),
          array("best probability per node", prob_view(prob)).hl([goal]) ], "return");
    leave(answer);
}
