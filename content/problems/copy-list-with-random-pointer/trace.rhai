// Build a chain of #{id, val, next} nodes from plain values.
fn chain(values) {
    let out = [];
    for i in 0..values.len() {
        out.push(#{
            id: i,
            val: values[i],
            next: if i + 1 < values.len() { i + 1 } else { () },
        });
    }
    out
}

// Same chain, but with a sentinel node in front so that removing the real head
// needs no special case. The sentinel is id 0 and the first real value is id 1.
fn chain_with_dummy(values) {
    let out = [#{ id: 0, val: "dummy", next: if values.len() > 0 { 1 } else { () } }];
    for i in 0..values.len() {
        out.push(#{
            id: i + 1,
            val: values[i],
            next: if i + 2 <= values.len() { i + 2 } else { () },
        });
    }
    out
}

fn order_of(nodes, head) {
    let out = [];
    let cur = head;
    let guard = 0;
    while type_of(cur) != "()" && guard < 200 {
        out.push(nodes[cur].val);
        cur = nodes[cur].next;
        guard += 1;
    }
    out
}

fn val_or_nil(nodes, id) {
    if type_of(id) == "()" { "nil" } else { `${nodes[id].val}` }
}

// Copy List With Random Pointer — two passes because of ordering, not cost.
//
// A single walk cannot work: node 0's random pointer may aim at node 4, which
// does not exist yet. Creating every clone first and only then wiring the links
// removes the ordering problem entirely — during the second pass every target
// is guaranteed to exist, so "translate this old pointer to a new one" is a
// plain map lookup with nil handled by the map returning nothing.

fn rand_labels(values, rnd) {
    let out = [];
    for i in 0..values.len() {
        out.push(if rnd[i] < 0 { "nil" } else { `${values[rnd[i]]}` });
    }
    out
}

fn clone_labels(values, made) {
    let out = [];
    for i in 0..values.len() {
        out.push(if made > i { `${values[i]}'` } else { "—" });
    }
    out
}

fn trace(input) {
    let values = input.list;
    let rnd = input.random;
    let n = values.len();
    let nodes = chain(values);
    let head = if n > 0 { 0 } else { () };

    enter("copyRandomList", #{ n: N(n) });

    let clones = map_new();
    step("init", "One map from each original node to its clone. It is what lets a pointer be translated without knowing the order things were built in.",
        [ list("original", nodes),
          array("random target of each node", rand_labels(values, rnd)),
          kv("original → clone", clones) ]);

    for i in 0..n {
        step("pass1", `Pass 1, node(${values[i]}): create its clone, and nothing else.`,
            [ list("original", nodes).pointers(#{ cur: i }),
              array("clones so far", clone_labels(values, i)),
              kv("original → clone", clones) ]);
        map_set(clones, `node(${values[i]})`, `clone(${values[i]})`);
        log(`clone node(${values[i]})`);
        step("make", `The clone holds the value but no links yet — its targets may not exist.`,
            [ list("original", nodes).pointers(#{ cur: i }),
              array("clones so far", clone_labels(values, i + 1)),
              kv("original → clone", clones).hl([`node(${values[i]})`]) ]);
    }

    let next_links = [];
    let rand_links = [];
    for i in 0..n {
        next_links.push("—");
        rand_links.push("—");
    }

    for i in 0..n {
        step("pass2", `Pass 2, node(${values[i]}): now every clone exists, so its links can be filled in.`,
            [ list("original", nodes).pointers(#{ cur: i }),
              array("clone.next", next_links), array("clone.random", rand_links) ]);

        next_links[i] = if i + 1 < n { `${values[i + 1]}'` } else { "nil" };
        step("linkNext", `clone(${values[i]}).next = ${next_links[i]} — the original's next translated through the map.`,
            [ list("original", nodes).pointers(#{ cur: i }),
              array("clone.next", next_links).hl([i]), array("clone.random", rand_links) ]);

        rand_links[i] = if rnd[i] < 0 { "nil" } else { `${values[rnd[i]]}'` };
        log(`clone(${values[i]}).random -> ${rand_links[i]}`);
        step("linkRandom", `clone(${values[i]}).random = ${rand_links[i]}. In pass 1 this target might not have existed; now it always does.`,
            [ list("original", nodes).pointers(#{ cur: i }),
              array("random target of each node", rand_labels(values, rnd)).hl([i]),
              array("clone.random", rand_links).hl([i]) ]);
    }

    let parts = [];
    for i in 0..n {
        parts.push(`${values[i]}:${if rnd[i] < 0 { "n" } else { `${rnd[i]}` }}`);
    }
    let answer = join(parts, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `The copy is independent of the original: ${answer} (value:random-index, n = nil).`,
        [ array("clone.next", next_links), array("clone.random", rand_links) ], "return");
    leave(answer);
}
