// Clone Graph — a map from original to copy, checked *before* recursing.
//
// Graphs have cycles, so a naive deep copy loops forever. Recording the copy
// the moment it is created — before wiring any neighbours — means a cycle
// coming back to a node finds the copy already there and stops.

fn parse_edges(spec) {
    let out = [];
    for pair in words(spec) {
        let parts = pair.split(",");
        if parts.len() == 2 { out.push([num(parts[0]), num(parts[1])]); }
    }
    out
}

fn neighbours_of(edges, id) {
    let out = [];
    for e in edges {
        if e[0] == id { out.push(e[1]); }
        if e[1] == id { out.push(e[0]); }
    }
    out
}

fn dfs(nodes, edges, id, seen, wired) {
    enter(`clone(${id})`, #{ node: N(id) });

    if map_has(seen, id) {
        step("seen", `Node ${id} has been cloned already — this is where a cycle would otherwise loop forever.`,
            [ graph("graph", nodes, edges).cur(id).done(map_keys(seen)) ]);
        step("retseen", "Return the existing copy.",
            [ graph("graph", nodes, edges).done(map_keys(seen)) ], "return");
        leave(`copy(${id})`);
        return [seen, wired];
    }

    step("create", `Node ${id} is new — make a copy of it.`,
        [ graph("graph", nodes, edges).cur(id).done(map_keys(seen)) ]);
    map_set(seen, id, `copy(${id})`);
    set(#{ seen: MAP(seen, [id]) });
    log(`cloned ${id}`);
    step("record",
        "Record the copy *before* touching its neighbours. That ordering is what makes cycles safe.",
        [ graph("graph", nodes, edges).cur(id).done(map_keys(seen)),
          kv("original → copy", seen).hl([id]) ]);

    for nb in neighbours_of(edges, id) {
        step("nbloop", `Node ${id} is connected to ${nb}.`,
            [ graph("graph", nodes, edges).cur(id).active([[id, nb]]).done(map_keys(seen)) ]);
        let out = dfs(nodes, edges, nb, seen, wired);
        seen = out[0];
        wired = out[1];
        wired.push([id, nb]);
        step("wire", `Link copy(${id}) to copy(${nb}).`,
            [ graph("graph", nodes, edges).cur(id).active(wired).done(map_keys(seen)) ]);
    }

    step("retcopy", `copy(${id}) is complete.`,
        [ graph("graph", nodes, edges).done(map_keys(seen)) ], "return");
    leave(`copy(${id})`);
    [seen, wired]
}

fn trace(input) {
    let edges = parse_edges(input.adj);
    let ids = [];
    for e in edges {
        if !ids.contains(e[0]) { ids.push(e[0]); }
        if !ids.contains(e[1]) { ids.push(e[1]); }
    }
    ids.sort(|a, b| if a < b { -1 } else if a > b { 1 } else { 0 });
    let nodes = [];
    for id in ids { nodes.push(#{ id: id, label: `${id}` }); }

    enter("cloneGraph", #{});
    if nodes.len() == 0 {
        step("nilcheck", "The graph is empty.", [ graph("graph", nodes, edges) ]);
        result("nil");
        step("retnil", "Return nil.", [ graph("graph", nodes, edges) ], "return");
        leave("nil");
        return;
    }
    step("nilcheck", `${nodes.len()} node(s) to copy.`, [ graph("graph", nodes, edges) ]);

    let seen = map_new();
    set(#{ seen: MAP(seen) });
    step("mapinit", "A map from each original node to its copy.",
        [ graph("graph", nodes, edges), kv("original → copy", seen) ]);
    step("call", `Start the depth-first copy at node ${ids[0]}.`,
        [ graph("graph", nodes, edges).cur(ids[0]) ], "call");

    let out = dfs(nodes, edges, ids[0], seen, []);
    seen = out[0];

    log(`${map_len(seen)} node(s) cloned`, "result");
    result(`copy(${ids[0]})`);
    step("call", `Every node was created exactly once — ${map_len(seen)} copies, cycles and all.`,
        [ graph("graph", nodes, edges).done(map_keys(seen)),
          kv("original → copy", seen) ], "return");
    leave(`copy(${ids[0]})`);
}
