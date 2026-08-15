fn parse_triples(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 3 { out.push([num(w[0]), num(w[1]), num(w[2])]); }
    }
    out
}

// Network Delay Time — Dijkstra, and why the closest node is already finished.
//
// The one idea worth holding on to: among the unsettled nodes, the one with
// the smallest tentative distance cannot be improved. Any alternative route to
// it would leave the settled region through some other unsettled node, and
// that node is at least as far away — so the detour is at least as long. That
// is what licenses settling it permanently and never revisiting it.

fn node_labels(n) {
    let out = [];
    for i in 1..(n + 1) { out.push(`${i}`); }
    out
}

fn edge_list(times) {
    let out = [];
    for t in times { out.push([t[0] - 1, t[1] - 1, true, t[2]]); }
    out
}

fn dist_view(dist, n) {
    let out = [];
    for v in 1..(n + 1) { out.push(if dist[v] >= 1000000 { "∞" } else { `${dist[v]}` }); }
    out
}

fn settled_ids(visited, n) {
    let out = [];
    for v in 1..(n + 1) { if visited[v] { out.push(v - 1); } }
    out
}

fn frontier_ids(dist, visited, n) {
    let out = [];
    for v in 1..(n + 1) { if !visited[v] && dist[v] < 1000000 { out.push(v - 1); } }
    out
}

fn trace(input) {
    let times = parse_triples(input.times);
    let n = input.n;
    let k = input.k;
    let nodes = node_labels(n);
    let edges = edge_list(times);

    enter("networkDelayTime", #{ n: N(n), k: N(k) });

    let adj = [];
    for i in 0..(n + 1) { adj.push([]); }
    for t in times { adj[t[0]].push([t[1], t[2]]); }
    step("build", `${plural(times.len(), "directed edge")}, each with a travel time.`,
        [ graph("network", nodes, edges) ]);

    let dist = [];
    for i in 0..(n + 1) { dist.push(1000000); }
    step("init", "Every node starts unreachable.",
        [ graph("network", nodes, edges),
          array("time to reach each node", dist_view(dist, n)) ]);

    dist[k] = 0;
    step("seed", `The signal starts at ${k}, so that node costs nothing to reach.`,
        [ graph("network", nodes, edges).cur(k - 1),
          array("time to reach each node", dist_view(dist, n)).hl([k - 1]) ]);

    let visited = [];
    for i in 0..(n + 1) { visited.push(false); }
    step("visited", "A settled node's distance is final and is never looked at again.",
        [ graph("network", nodes, edges) ]);

    for iter in 0..n {
        step("loop", `Round ${iter + 1}: settle one more node.`,
            [ graph("network", nodes, edges).done(settled_ids(visited, n))
                .frontier(frontier_ids(dist, visited, n)),
              array("time to reach each node", dist_view(dist, n)) ]);

        let u = -1;
        step("pick", "Which unsettled node is closest?",
            [ array("time to reach each node", dist_view(dist, n))
                .done(settled_ids(visited, n)) ]);

        for v in 1..(n + 1) {
            if !visited[v] && dist[v] < 1000000 && (u == -1 || dist[v] < dist[u]) { u = v; }
        }
        step("scan", `Reachable and unsettled: ${describe(dist, visited, n)}.`,
            [ graph("network", nodes, edges).done(settled_ids(visited, n))
                .frontier(frontier_ids(dist, visited, n)),
              array("time to reach each node", dist_view(dist, n)) ]);

        if u == -1 {
            step("stuck", "Nothing reachable is left unsettled — the rest of the network cannot be reached at all.",
                [ graph("network", nodes, edges).done(settled_ids(visited, n)) ]);
            break;
        }

        step("closest", `Node ${u}, at ${dist[u]}, is the closest. No route through any other unsettled node could beat that, because they are all at least as far away.`,
            [ graph("network", nodes, edges).cur(u - 1).done(settled_ids(visited, n)),
              array("time to reach each node", dist_view(dist, n)).hl([u - 1]) ]);

        visited[u] = true;
        log(`settle ${u} at ${dist[u]}`);
        step("settle", `So ${dist[u]} is final for node ${u}.`,
            [ graph("network", nodes, edges).done(settled_ids(visited, n)),
              array("time to reach each node", dist_view(dist, n)) ]);

        for e in adj[u] {
            step("relax", `Edge ${u} → ${e[0]}, taking ${e[1]}.`,
                [ graph("network", nodes, edges).cur(u - 1).active([[u - 1, e[0] - 1]])
                    .done(settled_ids(visited, n)) ]);

            if dist[u] + e[1] < dist[e[0]] {
                step("shorter", `${dist[u]} + ${e[1]} = ${dist[u] + e[1]}, better than ${if dist[e[0]] >= 1000000 { "unreachable" } else { `${dist[e[0]]}` }}.`,
                    [ array("time to reach each node", dist_view(dist, n)).ptr("to", e[0] - 1) ]);
                dist[e[0]] = dist[u] + e[1];
                log(`${e[0]} now reachable in ${dist[e[0]]}`);
                step("update", `Node ${e[0]} is now reachable in ${dist[e[0]]}.`,
                    [ array("time to reach each node", dist_view(dist, n)).hl([e[0] - 1]) ]);
            } else {
                step("shorter", `${dist[u]} + ${e[1]} = ${dist[u] + e[1]} is no better than the ${dist[e[0]]} already known.`,
                    [ array("time to reach each node", dist_view(dist, n)).ptr("to", e[0] - 1) ]);
            }
        }
    }

    let ans = 0;
    step("max", "The signal has arrived everywhere only once the slowest node has heard it.",
        [ array("time to reach each node", dist_view(dist, n)) ]);

    let failed = false;
    for v in 1..(n + 1) {
        step("check", `Node ${v}: ${if dist[v] >= 1000000 { "unreachable" } else { `${dist[v]}` }}.`,
            [ array("time to reach each node", dist_view(dist, n)).ptr("v", v - 1) ]);

        if dist[v] >= 1000000 {
            failed = true;
            log(`node ${v} unreachable`, "result");
            step("unreachable", `Node ${v} was never reached, so the signal never covers the network.`,
                [ graph("network", nodes, edges).bad([v - 1]) ]);
            step("fail", "Return -1.", [ text("answer", ["-1"]) ], "return");
            break;
        }
        if dist[v] > ans {
            ans = dist[v];
            step("bump", `New slowest arrival: ${ans}.`,
                [ array("time to reach each node", dist_view(dist, n)).hl([v - 1]) ]);
        } else {
            step("bump", `${dist[v]} does not beat the slowest so far, ${ans}.`,
                [ array("time to reach each node", dist_view(dist, n)) ]);
        }
    }

    if failed {
        result("-1");
        leave("-1");
        return;
    }

    log(`answer ${ans}`, "result");
    result(`${ans}`);
    step("ret", `Every node hears the signal within ${ans}.`,
        [ graph("network", nodes, edges).done(settled_ids(visited, n)),
          array("time to reach each node", dist_view(dist, n)) ], "return");
    leave(`${ans}`);
}

fn describe(dist, visited, n) {
    let out = [];
    for v in 1..(n + 1) {
        if !visited[v] && dist[v] < 1000000 { out.push(`${v} at ${dist[v]}`); }
    }
    if out.len() == 0 { "nothing" } else { join(out, ", ") }
}
