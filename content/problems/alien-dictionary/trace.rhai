// Alien Dictionary — each adjacent pair of words is worth exactly one fact.
//
// The comparison stops at the first difference and everything after it is
// uninformative: "wrt" before "wrf" says t comes before f, and says nothing
// whatever about w or r. Collect those single facts as edges and any
// topological order of the result is a valid alphabet. Two failure modes: a
// cycle among the constraints, and a word placed before its own prefix — which
// no letter ordering could ever justify, so it is caught separately.

fn order_view(indeg) {
    let out = [];
    for e in indeg { out.push(`${e[0]}: ${e[1]}`); }
    out
}

fn edge_view(adj) {
    let out = [];
    for e in adj {
        if e[1].len() > 0 { out.push(`${e[0]} before ${join(e[1], ", ")}`); }
    }
    if out.len() == 0 { out.push("(no constraints yet)"); }
    out
}

fn trace(input) {
    let ws = words(input.words);

    enter("alienOrder", #{ words: N(ws.len()) });

    let adj = map_new();
    step("init", "For each letter, the letters it must come before.",
        [ array("words, in alien order", ws), text("constraints", edge_view(adj)) ]);

    let indeg = map_new();
    step("degrees", "And for each letter, how many constraints still point at it.",
        [ text("unmet predecessors", order_view(indeg)) ]);

    for w in ws {
        for c in chars(w) {
            if !map_has(indeg, c) {
                map_set(indeg, c, 0);
                map_set(adj, c, []);
            }
        }
    }
    step("letters", `${plural(map_len(indeg), "distinct letter")}: ${join(map_keys(indeg), ", ")}. Every one of them must appear in the answer, even if nothing constrains it.`,
        [ text("unmet predecessors", order_view(indeg)) ]);

    for i in 0..(ws.len() - 1) {
        let a = ws[i];
        let b = ws[i + 1];
        let ca = chars(a);
        let cb = chars(b);
        let n = if ca.len() < cb.len() { ca.len() } else { cb.len() };

        step("pairs", `Comparing "${a}" with "${b}".`,
            [ array("words, in alien order", ws).hl([i, i + 1]),
              text("constraints", edge_view(adj)) ]);

        let same_prefix = true;
        for j in 0..n { if ca[j] != cb[j] { same_prefix = false; } }

        if ca.len() > cb.len() && same_prefix {
            log("a word precedes its own prefix", "result");
            step("prefix", `"${a}" comes before "${b}", but "${b}" is a prefix of "${a}" — in any alphabet the shorter word sorts first, so this list cannot be sorted at all.`,
                [ array("words, in alien order", ws).bad([i, i + 1]) ]);
            step("invalid", "No ordering exists.", [ text("answer", ["(no valid order)"]) ], "return");
            result("(no valid order)");
            leave("(no valid order)");
            return;
        }
        step("prefix", "Neither word is a prefix of the other in the wrong direction.",
            [ array("words, in alien order", ws).hl([i, i + 1]) ]);

        let found = false;
        for j in 0..n {
            step("compare", `Position ${j}: "${ca[j]}" against "${cb[j]}".`,
                [ array("first word", ca).ptr("j", j), array("second word", cb).ptr("j", j) ]);

            if ca[j] != cb[j] {
                found = true;
                step("differ", `They differ, so "${ca[j]}" must come before "${cb[j]}" — and nothing after this position tells us anything at all.`,
                    [ array("first word", ca).hl([j]), array("second word", cb).hl([j]) ]);

                let outs = map_get(adj, ca[j]);
                let known = false;
                for x in outs { if x == cb[j] { known = true; } }

                if !known {
                    outs.push(cb[j]);
                    map_set(adj, ca[j], outs);
                    step("edge", `Record "${ca[j]}" before "${cb[j]}".`,
                        [ text("constraints", edge_view(adj)) ]);
                    map_inc(indeg, cb[j], 1);
                    log(`${ca[j]} before ${cb[j]}`);
                    step("bump", `"${cb[j]}" now has ${map_get(indeg, cb[j])} unmet predecessor(s).`,
                        [ text("unmet predecessors", order_view(indeg)) ]);
                } else {
                    step("edge", "That constraint is already known.",
                        [ text("constraints", edge_view(adj)) ]);
                    step("bump", "Nothing changes.", [ text("unmet predecessors", order_view(indeg)) ]);
                }

                step("stop", "Stop comparing this pair.",
                    [ array("first word", ca), array("second word", cb) ]);
                break;
            }
        }
        if !found {
            step("compare", "The two words agree over their whole common prefix, so this pair says nothing.",
                [ array("words, in alien order", ws).hl([i, i + 1]) ]);
        }
    }

    let q = [];
    step("queue", "Letters that may be emitted next.", [ queue("ready", q) ]);

    for e in indeg {
        if e[1] == 0 { q.push(e[0]); }
    }
    step("seed", `${plural(q.len(), "letter")} start with nothing pointing at them.`,
        [ queue("ready", q), text("unmet predecessors", order_view(indeg)) ]);

    let out = [];
    step("build", "The alphabet, in order.", [ text("alphabet so far", [join(out, "")]) ]);

    let qi = 0;
    while qi < q.len() {
        step("loop", `${q.len() - qi} letter(s) ready.`,
            [ queue("ready", slice_from(q, qi)), text("alphabet so far", [join(out, "")]) ]);

        let c = q[qi];
        qi += 1;
        step("pop", `Take "${c}".`,
            [ queue("ready", slice_from(q, qi)) ]);

        out.push(c);
        step("emit", `Alphabet so far: "${join(out, "")}".`,
            [ text("alphabet so far", [join(out, "")]) ]);

        for nxt in map_get(adj, c) {
            step("neighbours", `"${c}" must come before "${nxt}".`,
                [ text("constraints", edge_view(adj)) ]);
            map_inc(indeg, nxt, -1);
            step("dec", `"${nxt}" now has ${map_get(indeg, nxt)} unmet predecessor(s).`,
                [ text("unmet predecessors", order_view(indeg)) ]);
            if map_get(indeg, nxt) == 0 {
                step("ready", `Nothing is left before "${nxt}".`,
                    [ text("unmet predecessors", order_view(indeg)) ]);
                q.push(nxt);
                step("enqueue", `Queue "${nxt}".`, [ queue("ready", slice_from(q, qi)) ]);
            } else {
                step("ready", `"${nxt}" is still waiting on something else.`,
                    [ text("unmet predecessors", order_view(indeg)) ]);
            }
        }
    }

    if out.len() != map_len(indeg) {
        log("constraints contain a cycle", "result");
        step("cycle", `Only ${out.len()} of ${map_len(indeg)} letters could be emitted — the remaining ones point at each other in a cycle.`,
            [ text("alphabet so far", [join(out, "")]),
              text("unmet predecessors", order_view(indeg)) ]);
        step("none", "So no consistent alphabet exists.",
            [ text("answer", ["(no valid order)"]) ], "return");
        result("(no valid order)");
        leave("(no valid order)");
        return;
    }

    let answer = join(out, "");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Every letter placed with all its constraints satisfied: ${answer}.`,
        [ text("alphabet so far", [answer]) ], "return");
    leave(answer);
}

fn slice_from(q, qi) {
    let out = [];
    for k in qi..q.len() { out.push(q[k]); }
    out
}
