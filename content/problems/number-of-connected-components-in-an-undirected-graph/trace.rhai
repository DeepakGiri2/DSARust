// Number of Connected Components — union-find, counting downwards.
//
// Starting from n separate components and merging is easier than discovering
// groups: each edge either joins two different groups, which costs one from
// the count, or connects things that were already together, which changes
// nothing.

fn find_root(parent, x) {
    while parent[x] != x { x = parent[x]; }
    x
}

fn nodes_of(n) {
    let out = [];
    for i in 0..n { out.push(#{ id: i, label: `${i}` }); }
    out
}

fn trace(input) {
    let n = input.n;
    let edges = [];
    for pair in words(input.edges) {
        let parts = pair.split(",");
        if parts.len() == 2 { edges.push([num(parts[0]), num(parts[1])]); }
    }
    let nodes = nodes_of(n);

    let parent = [];
    for i in 0..n { parent.push(i); }

    enter("countComponents", #{ n: N(n) });
    step("init", "Union-find: parent[x] points towards the representative of x's group.",
        [ graph("graph", nodes, []), array("parent", parent) ]);

    for i in 0..n {
        step("seed", `Node ${i}.`, [ array("parent", parent).ptr("i", i) ]);
        step("self", `It starts as its own group.`, [ array("parent", parent).hl([i]) ]);
    }

    let count = n;
    set(#{ count: N(count) });
    step("count", `${n} separate components to begin with.`,
        [ graph("graph", nodes, []), array("parent", parent) ]);

    let wired = [];
    for e in edges {
        wired.push([e[0], e[1]]);
        step("loop", `Edge ${e[0]} — ${e[1]}.`,
            [ graph("graph", nodes, wired).active([[e[0], e[1]]]),
              array("parent", parent) ]);

        let a = find_root(parent, e[0]);
        let b = find_root(parent, e[1]);
        step("find", `Their group representatives are ${a} and ${b}.`,
            [ graph("graph", nodes, wired).frontier([a, b]),
              array("parent", parent).hl([a, b]) ]);

        if a != b {
            step("check", `Different groups, so this edge really joins something.`,
                [ graph("graph", nodes, wired).active([[e[0], e[1]]]) ]);
            parent[a] = b;
            step("union", `Point group ${a} at group ${b}.`,
                [ array("parent", parent).hl([a]) ]);
            count -= 1;
            set(#{ count: N(count) });
            log(`merged ${a} into ${b}, ${count} left`);
            step("merge", `${count} component(s) left.`,
                [ graph("graph", nodes, wired), array("parent", parent) ]);
        } else {
            step("check", `Both are already in group ${a} — this edge adds nothing new.`,
                [ graph("graph", nodes, wired).done([a]) ]);
        }
    }

    log(`answer ${count}`, "result");
    result(`${count}`);
    step("ret", `${count} connected component(s).`,
        [ graph("graph", nodes, wired), array("parent", parent) ], "return");
    leave(`${count}`);
}
