// Min Cost to Connect All Points — Prim, justified by the cut property.
//
// At every step there is a boundary between the points already connected and
// those outside. The cheapest edge crossing that boundary is always in some
// minimum spanning tree — swap it into any tree that lacks it and the total
// cannot go up. So growing greedily across the cheapest crossing edge is not a
// heuristic, it is provably optimal. Since every pair of points is connectable,
// one number per outside point ("cheapest link into the tree so far") captures
// the whole boundary.

fn parse_points(spec) {
    let out = [];
    for p in words(spec) {
        let xy = p.split(",");
        out.push([num(xy[0]), num(xy[1])]);
    }
    out
}

fn abs_of(v) { if v < 0 { -v } else { v } }

fn labels(points) {
    let out = [];
    for p in points { out.push(`(${p[0]},${p[1]})`); }
    out
}

fn best_view(best) {
    let out = [];
    for v in best { out.push(if v >= 1000000 { "∞" } else { `${v}` }); }
    out
}

fn in_ids(inTree) {
    let out = [];
    for i in 0..inTree.len() { if inTree[i] { out.push(i); } }
    out
}

fn out_ids(inTree) {
    let out = [];
    for i in 0..inTree.len() { if !inTree[i] { out.push(i); } }
    out
}

fn trace(input) {
    let points = parse_points(input.points);
    let n = points.len();
    let names = labels(points);

    enter("minCostConnectPoints", #{ n: N(n) });

    let in_tree = [];
    for i in 0..n { in_tree.push(false); }
    step("init", "The tree starts empty; every point is outside it.",
        [ array("points", names) ]);

    let best = [];
    for i in 0..n { best.push(1000000); }
    step("dist", "For each outside point, the cheapest known link into the tree.",
        [ array("points", names), array("cheapest link into the tree", best_view(best)) ]);

    best[0] = 0;
    step("seed", "Start the tree at the first point — which point does not matter, since every spanning tree must contain them all.",
        [ array("cheapest link into the tree", best_view(best)).hl([0]) ]);

    let total = 0;
    set(#{ total: N(total) });
    step("total", "Running cost.",
        [ array("cheapest link into the tree", best_view(best)) ]);

    for iter in 0..n {
        step("loop", `${plural(iter, "point")} connected so far.`,
            [ array("points", names).done(in_ids(in_tree)),
              array("cheapest link into the tree", best_view(best)) ]);

        let u = -1;
        step("pick", "Which outside point is cheapest to attach?",
            [ array("cheapest link into the tree", best_view(best)).done(in_ids(in_tree)) ]);

        for v in 0..n {
            if !in_tree[v] && (u == -1 || best[v] < best[u]) { u = v; }
        }
        step("scan", `Boundary costs: ${crossing(best, in_tree, names)}.`,
            [ array("cheapest link into the tree", best_view(best)).done(in_ids(in_tree)) ]);

        step("closest", `${names[u]} costs ${best[u]} — the cheapest edge across the boundary, so it belongs to a minimum spanning tree.`,
            [ array("points", names).hl([u]).done(in_ids(in_tree)),
              array("cheapest link into the tree", best_view(best)).hl([u]) ]);

        in_tree[u] = true;
        step("add", `${names[u]} joins the tree.`,
            [ array("points", names).done(in_ids(in_tree)) ]);

        total += best[u];
        set(#{ total: N(total) });
        log(`connect ${names[u]} for ${best[u]}; total ${total}`);
        step("pay", `Total cost is now ${total}.`,
            [ text("total", [`${total}`]) ]);

        for v in 0..n {
            if in_tree[v] { continue; }
            step("relax", `Could ${names[u]} be a cheaper way to reach ${names[v]}?`,
                [ array("points", names).pointers(#{ from: u, to: v }).done(in_ids(in_tree)) ]);

            let d = abs_of(points[u][0] - points[v][0]) + abs_of(points[u][1] - points[v][1]);
            step("dist2", `Manhattan distance ${names[u]} to ${names[v]} is ${d}.`,
                [ array("cheapest link into the tree", best_view(best)).ptr("v", v) ]);

            if d < best[v] {
                step("cheaper", `${d} beats the ${if best[v] >= 1000000 { "nothing" } else { `${best[v]}` }} known before.`,
                    [ array("cheapest link into the tree", best_view(best)).ptr("v", v) ]);
                best[v] = d;
                step("update", `${names[v]} is now ${d} away from the tree.`,
                    [ array("cheapest link into the tree", best_view(best)).hl([v]) ]);
            } else {
                step("cheaper", `${d} is no better than the ${best[v]} already recorded.`,
                    [ array("cheapest link into the tree", best_view(best)).ptr("v", v) ]);
            }
        }
    }

    log(`answer ${total}`, "result");
    result(`${total}`);
    step("ret", `Connecting all ${n} points costs ${total}.`,
        [ array("points", names).done(in_ids(in_tree)), text("total", [`${total}`]) ], "return");
    leave(`${total}`);
}

fn crossing(best, in_tree, names) {
    let out = [];
    for v in 0..best.len() {
        if !in_tree[v] && best[v] < 1000000 { out.push(`${names[v]} at ${best[v]}`); }
    }
    if out.len() == 0 { "nothing yet" } else { join(out, ", ") }
}
