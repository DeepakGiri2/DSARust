// Serialize and Deserialize Binary Tree — preorder with explicit nulls.
//
// Preorder alone cannot rebuild a tree, because the shape is lost. Writing a
// marker for every missing child restores it: the decoder then knows exactly
// when a branch ends, and one left-to-right pass is enough.

fn ser(nodes, id, out) {
    if type_of(id) == "()" {
        enter("serialize(nil)");
        step("snull", "An absent child.", [ tree("tree", nodes) ]);
        out.push("n");
        step("sapnull", `Write the marker "n" — without it the shape could not be recovered.`,
            [ tree("tree", nodes), text("serialized", [join(out, " ")]) ]);
        leave("");
        return out;
    }

    let val = nodes[id].val;
    enter(`serialize(${val})`);
    out.push(`${val}`);
    step("sapval", `Write ${val} — root first, which is what makes it preorder.`,
        [ tree("tree", nodes).cur(id), text("serialized", [join(out, " ")]) ]);

    step("sleft", `Serialize the left subtree of ${val}.`,
        [ tree("tree", nodes).cur(id) ], "call");
    out = ser(nodes, nodes[id].left, out);
    step("sright", `Now the right subtree of ${val}.`,
        [ tree("tree", nodes).cur(id) ], "call");
    out = ser(nodes, nodes[id].right, out);

    leave("");
    out
}

fn de(tokens, state, built) {
    let tok = tokens[state.i];
    enter(`deserialize(${tok})`, #{ i: N(state.i) });
    step("dread", `Read "${tok}" from position ${state.i}.`,
        [ text("serialized", [join(tokens, " ")]),
          tree("rebuilt", built.nodes) ]);
    state.i += 1;

    if tok == "n" {
        step("dnull", "A marker — this branch ends here. No guessing required.",
            [ tree("rebuilt", built.nodes) ]);
        leave("nil");
        return [state, built, ()];
    }

    built.nodes.push(#{ id: built.nodes.len(), val: num(tok), left: (), right: () });
    let me = built.nodes.len() - 1;
    step("dnode", `Create node(${tok}).`,
        [ tree("rebuilt", built.nodes).cur(me) ]);

    step("dleft", `Everything that follows belongs to its left subtree, until a marker closes it.`,
        [ tree("rebuilt", built.nodes).cur(me) ], "call");
    let l = de(tokens, state, built);
    state = l[0];
    built = l[1];
    built.nodes[me].left = l[2];

    step("dright", `Then the right subtree of ${tok}.`,
        [ tree("rebuilt", built.nodes).cur(me) ], "call");
    let r = de(tokens, state, built);
    state = r[0];
    built = r[1];
    built.nodes[me].right = r[2];

    leave(`node(${tok})`);
    [state, built, me]
}

fn trace(input) {
    let nodes = tree_nodes(input.root);

    enter("serialize", #{});
    step("sinit", "Walk the tree in preorder, writing a marker for every missing child.",
        [ tree("tree", nodes) ]);
    let out = ser(nodes, if nodes.len() > 0 { 0 } else { () }, []);
    let text = join(out, " ");
    log(`serialized: ${text}`, "result");
    step("sjoin", `The tree becomes "${text}".`,
        [ tree("tree", nodes), text("serialized", [text]) ], "return");
    leave(text);

    enter("deserialize", #{ data: S(text) });
    step("dsplit", "Split it back into tokens and rebuild left to right.",
        [ text("serialized", [text]) ]);
    let built = #{ nodes: [] };
    let out2 = de(out, #{ i: 0 }, built);
    built = out2[1];

    result(text);
    step("dret", "The rebuilt tree is identical to the original — the markers carried the shape.",
        [ tree("rebuilt", built.nodes) ], "return");
    leave("root");
}
