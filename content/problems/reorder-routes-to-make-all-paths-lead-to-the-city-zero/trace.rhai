fn parse_pairs(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

// Reorder Routes — walk the tree as if undirected, and charge for direction.
//
// Following the roads as given cannot work: they may point away from 0. So the
// tree is walked as an undirected graph, with each road stored in both
// directions and tagged. Every edge crossed while moving *away* from 0 needs
// to point back towards it, so a road being traversed in its original
// direction is pointing the wrong way and costs one reversal.

fn build_adj(n, pairs) {
    let adj = [];
    for i in 0..n { adj.push([]); }
    for c in pairs {
        adj[c[0]].push([c[1], 1]);
        adj[c[1]].push([c[0], 0]);
    }
    adj
}

fn node_labels(n) {
    let out = [];
    for i in 0..n { out.push(`${i}`); }
    out
}

fn dir_edges(pairs) {
    let out = [];
    for c in pairs { out.push([c[0], c[1], true]); }
    out
}

fn dfs(adj, visited, node, pairs, flipped) {
    enter(`dfs(${node})`, #{ node: N(node) });

    visited[node] = true;
    step("visit", `City ${node} is now reachable from 0.`,
        [ graph("roads (arrows point the way they may be driven)", node_labels(visited.len()), dir_edges(pairs))
            .cur(node).done(reached(visited)).bad(flipped) ]);

    let total = 0;
    step("total", "Count the reversals needed in this subtree.",
        [ graph("roads (arrows point the way they may be driven)", node_labels(visited.len()), dir_edges(pairs))
            .cur(node).done(reached(visited)).bad(flipped) ]);

    for e in adj[node] {
        step("neighbours", `Neighbour ${e[0]}.`,
            [ graph("roads (arrows point the way they may be driven)", node_labels(visited.len()), dir_edges(pairs))
                .cur(node).frontier([e[0]]).done(reached(visited)).bad(flipped) ]);

        if visited[e[0]] {
            step("seen", `${e[0]} is already reached — in a tree that can only be the city we came from.`,
                [ graph("roads (arrows point the way they may be driven)", node_labels(visited.len()), dir_edges(pairs))
                    .cur(node).done(reached(visited)).bad(flipped) ]);
            continue;
        }

        if e[1] == 1 {
            flipped.push(e[0]);
            log(`reverse ${node} -> ${e[0]}`);
        }
        total += e[1];
        step("cost",
            if e[1] == 1 {
                `The road runs ${node} → ${e[0]}, away from city 0 — it must be reversed. Cost so far ${total}.`
            } else {
                `The road already runs ${e[0]} → ${node}, towards city 0. Nothing to do. Cost so far ${total}.`
            },
            [ graph("roads (arrows point the way they may be driven)", node_labels(visited.len()), dir_edges(pairs))
                .cur(node).active([[node, e[0]]]).done(reached(visited)).bad(flipped) ]);

        step("recurse", `Continue outwards to ${e[0]}.`,
            [ graph("roads (arrows point the way they may be driven)", node_labels(visited.len()), dir_edges(pairs))
                .cur(e[0]).done(reached(visited)).bad(flipped) ], "call");
        let out = dfs(adj, visited, e[0], pairs, flipped);
        visited = out[0];
        total += out[1];
        flipped = out[2];
    }

    step("ret2", `Subtree at ${node} needs ${plural(total, "reversal")}.`,
        [ graph("roads (arrows point the way they may be driven)", node_labels(visited.len()), dir_edges(pairs))
            .done(reached(visited)).bad(flipped) ], "return");
    leave(`${total}`);
    [visited, total, flipped]
}

fn reached(visited) {
    let out = [];
    for i in 0..visited.len() { if visited[i] { out.push(i); } }
    out
}

fn trace(input) {
    let n = input.n;
    let pairs = parse_pairs(input.connections);

    enter("minReorder", #{ n: N(n) });

    let adj = [];
    for i in 0..n { adj.push([]); }
    step("build", "An adjacency list that ignores direction — but remembers it.",
        [ graph("roads (arrows point the way they may be driven)", node_labels(n), dir_edges(pairs)) ]);

    for c in pairs {
        step("edges", `Road ${c[0]} → ${c[1]}.`,
            [ graph("roads (arrows point the way they may be driven)", node_labels(n), dir_edges(pairs))
                .active([[c[0], c[1]]]) ]);
        adj[c[0]].push([c[1], 1]);
        step("forward", `Store it at ${c[0]} with a cost of 1: driving this way is against the required direction.`,
            [ graph("roads (arrows point the way they may be driven)", node_labels(n), dir_edges(pairs))
                .active([[c[0], c[1]]]) ]);
        adj[c[1]].push([c[0], 0]);
        step("backward", `And at ${c[1]} with a cost of 0: that direction already points towards where we came from.`,
            [ graph("roads (arrows point the way they may be driven)", node_labels(n), dir_edges(pairs))
                .active([[c[0], c[1]]]) ]);
    }

    let visited = [];
    for i in 0..n { visited.push(false); }
    step("init", "Nothing reached yet.",
        [ graph("roads (arrows point the way they may be driven)", node_labels(n), dir_edges(pairs)) ]);

    step("call", "Walk outwards from city 0. Every edge crossed is an edge that must lead back here.",
        [ graph("roads (arrows point the way they may be driven)", node_labels(n), dir_edges(pairs)).cur(0) ],
        "call");
    let out = dfs(adj, visited, 0, pairs, []);
    let count = out[1];

    log(`answer ${count}`, "result");
    result(`${count}`);
    step("ret", `${plural(count, "road")} must be reversed.`,
        [ graph("roads (arrows point the way they may be driven)", node_labels(n), dir_edges(pairs))
            .bad(out[2]) ], "return");
    leave(`${count}`);
}
