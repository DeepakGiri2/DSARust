// Evaluate Division — a/b = 2 is an edge, and a quotient is a path product.
//
// The insight is the modelling, not the search. Once a/b = 2 is read as
// "a → b weighted 2, and b → a weighted 1/2", a query is a walk that
// multiplies weights, and a/a = 1 falls out of the empty walk. Everything
// after that is a textbook DFS: mark visited, recurse, bubble the first
// success back up.

const G = "graph";

fn parse_equations(spec) {
    let out = [];
    for part in spec.split(",") {
        let t = part;
        t.trim();
        let sides = t.split("=");
        if sides.len() < 2 { continue; }
        let vars = sides[0].split("/");
        if vars.len() < 2 { continue; }
        let a = vars[0]; a.trim();
        let b = vars[1]; b.trim();
        let v = sides[1]; v.trim();
        out.push([a, b, num(v).to_float()]);
    }
    out
}

fn parse_queries(spec) {
    let out = [];
    for part in spec.split(",") {
        let t = part;
        t.trim();
        let vars = t.split("/");
        if vars.len() < 2 { continue; }
        let a = vars[0]; a.trim();
        let b = vars[1]; b.trim();
        out.push([a, b]);
    }
    out
}

fn fmt2(x) {
    if x < 0.0 { return "-1"; }
    let scaled = (x * 100.0 + 0.5).to_int();
    let whole = scaled / 100;
    let frac = scaled % 100;
    `${whole}.${if frac < 10 { "0" } else { "" }}${frac}`
}

fn node_index(names, want) {
    for i in 0..names.len() {
        if names[i] == want { return i; }
    }
    -1
}

fn edge_list(eqs, names) {
    let out = [];
    for e in eqs {
        out.push([node_index(names, e[0]), node_index(names, e[1]), true, fmt2(e[2])]);
    }
    out
}

fn neighbours(eqs, node) {
    // [other, weight] for both directions of every equation touching `node`.
    let out = [];
    for e in eqs {
        if e[0] == node { out.push([e[1], e[2]]); }
        if e[1] == node { out.push([e[0], 1.0 / e[2]]); }
    }
    out
}

fn search(eqs, names, cur, target, acc, seen) {
    enter(`dfs(${cur} → ${target})`, #{ cur: S(cur), product: S(fmt2(acc)) });

    if cur == target {
        step("arrive", `Arrived at ${target}.`,
            [ graph(global::G, names, edge_list(eqs, names)).cur(node_index(names, cur)) ]);
        step("hit", `The product along the path is ${fmt2(acc)} — that is the quotient.`,
            [ graph(global::G, names, edge_list(eqs, names)).done([node_index(names, cur)]) ],
            "return");
        leave(fmt2(acc));
        return [seen, acc];
    }
    step("arrive", `${cur} is not ${target} yet; the running product is ${fmt2(acc)}.`,
        [ graph(global::G, names, edge_list(eqs, names)).cur(node_index(names, cur)) ]);

    set_add(seen, cur);
    step("mark", `Mark ${cur} visited so a cycle cannot send the search round forever.`,
        [ graph(global::G, names, edge_list(eqs, names)).cur(node_index(names, cur)) ]);

    for nb in neighbours(eqs, cur) {
        let next = nb[0];
        let w = nb[1];
        step("neighbor", `${cur} / ${next} = ${fmt2(w)}, so stepping there multiplies the product by ${fmt2(w)}.`,
            [ graph(global::G, names, edge_list(eqs, names))
                  .cur(node_index(names, cur)).frontier([node_index(names, next)])
                  .active([[node_index(names, cur), node_index(names, next)]]) ]);

        if set_has(seen, next) {
            step("skip", `${next} has already been visited on this path — skip it.`,
                [ graph(global::G, names, edge_list(eqs, names)).cur(node_index(names, cur)) ]);
            continue;
        }

        step("recurse", `Follow the edge to ${next} carrying ${fmt2(acc)} × ${fmt2(w)} = ${fmt2(acc * w)}.`,
            [ graph(global::G, names, edge_list(eqs, names)).cur(node_index(names, next)) ],
            "call");
        let got = search(eqs, names, next, target, acc * w, seen);
        seen = got[0];
        if got[1] >= 0.0 {
            step("bubble", `A path was found below — pass ${fmt2(got[1])} straight back up.`,
                [ graph(global::G, names, edge_list(eqs, names)).done([node_index(names, cur)]) ],
                "return");
            leave(fmt2(got[1]));
            return [seen, got[1]];
        }
    }

    step("dead", `No neighbour of ${cur} leads to ${target}. Dead end.`,
        [ graph(global::G, names, edge_list(eqs, names)).bad([node_index(names, cur)]) ],
        "return");
    leave("-1");
    [seen, -1.0]
}

fn trace(input) {
    let eqs = parse_equations(input.equations);
    let queries = parse_queries(input.queries);

    let names = [];
    for e in eqs {
        if node_index(names, e[0]) < 0 { names.push(e[0]); }
        if node_index(names, e[1]) < 0 { names.push(e[1]); }
    }

    enter("calcEquation", #{ equations: N(eqs.len()), queries: N(queries.len()) });
    step("build", `${plural(eqs.len(), "equation")} over ${plural(names.len(), "variable")}. Each one becomes two directed edges.`,
        [ graph(global::G, names, []) ]);

    for e in eqs {
        step("edge", `${e[0]} / ${e[1]} = ${fmt2(e[2])}.`,
            [ graph(global::G, names, edge_list(eqs, names))
                  .frontier([node_index(names, e[0]), node_index(names, e[1])]) ]);
        step("fwd", `${e[0]} → ${e[1]} with weight ${fmt2(e[2])}.`,
            [ graph(global::G, names, edge_list(eqs, names))
                  .active([[node_index(names, e[0]), node_index(names, e[1])]]) ]);
        step("rev", `And ${e[1]} → ${e[0]} with weight 1 / ${fmt2(e[2])} = ${fmt2(1.0 / e[2])}. Without the reverse edge, b / a would be unanswerable.`,
            [ graph(global::G, names, edge_list(eqs, names))
                  .active([[node_index(names, e[1]), node_index(names, e[0])]]) ]);
    }

    let answers = [];
    step("ans", "The graph is built. Every query is now a path product.",
        [ graph(global::G, names, edge_list(eqs, names)) ]);

    for q in queries {
        set(#{ query: S(`${q[0]} / ${q[1]}`) });
        step("query", `Query: ${q[0]} / ${q[1]}.`,
            [ graph(global::G, names, edge_list(eqs, names)) ]);

        if node_index(names, q[0]) < 0 || node_index(names, q[1]) < 0 {
            step("unknown", `${if node_index(names, q[0]) < 0 { q[0] } else { q[1] }} never appears in any equation, so nothing can be said about it — not even x / x.`,
                [ graph(global::G, names, edge_list(eqs, names)) ]);
            answers.push("-1");
            step("nope", "Answer -1.", [ text("answers", answers) ]);
            continue;
        }

        step("search", `Both variables are known — search for a path from ${q[0]} to ${q[1]}, starting with a product of 1.`,
            [ graph(global::G, names, edge_list(eqs, names)).cur(node_index(names, q[0])) ],
            "call");
        let seen = set_new();
        let got = search(eqs, names, q[0], q[1], 1.0, seen);
        answers.push(fmt2(got[1]));
        log(`${q[0]} / ${q[1]} = ${fmt2(got[1])}`, "result");
    }

    let out = join(answers, " ");
    result(out);
    step("ret", `Answers: ${out}.`, [ text("answers", answers) ], "return");
    leave(out);
}
