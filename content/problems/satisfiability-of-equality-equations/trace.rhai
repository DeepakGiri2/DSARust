// Satisfiability of Equality Equations — order the passes, and it is free.
//
// Do the two passes in the wrong order and the algorithm is simply wrong: a
// != checked before a later == has no way to know what that == will merge.
// Equalities are unconditional facts, so they all go in first; inequalities
// are questions asked of the finished structure.

const PARENT = "parent of each letter (a … z as 0 … 25)";

fn find_root(parent, x) {
    enter(`find(${x})`, #{ x: N(x) });
    let cur = x;
    while parent[cur] != cur {
        step("walk", `${cur} is not its own root — its parent is ${parent[cur]}.`,
            [ array(global::PARENT, parent).ptr("x", cur).hl([cur]) ]);
        parent[cur] = parent[parent[cur]];
        step("compress", `Point ${cur} straight at ${parent[cur]}, halving the path for every later find.`,
            [ array(global::PARENT, parent).ptr("x", cur).hl([cur]) ]);
        cur = parent[cur];
        step("up", `Climb to ${cur}.`, [ array(global::PARENT, parent).ptr("x", cur) ]);
    }
    step("root", `${cur} is its own parent — the root, and therefore the name of its whole component.`,
        [ array(global::PARENT, parent).ptr("root", cur).hl([cur]) ], "return");
    leave(`${cur}`);
    [parent, cur]
}

fn parse_equations(spec) {
    let out = [];
    for part in spec.split(",") {
        let t = part;
        t.trim();
        if t.len() >= 4 { out.push(t); }
    }
    out
}

fn letters_of(eqs) {
    // The letters actually mentioned, so the picture stays small.
    let seen = set_new();
    for e in eqs {
        set_add(seen, e[0].to_string());
        set_add(seen, e[3].to_string());
    }
    set_items(seen)
}

fn roots_map(parent, letters) {
    let m = map_new();
    for ch in letters {
        let i = ord(ch) - ord("a");
        let cur = i;
        while parent[cur] != cur { cur = parent[cur]; }
        map_set(m, ch, chr(cur + ord("a")));
    }
    m
}

const ROOTS = "letter → the component it belongs to";

fn trace(input) {
    let eqs = parse_equations(input.equations);
    let letters = letters_of(eqs);

    enter("equationsPossible", #{ equations: N(eqs.len()) });

    let parent = [];
    for i in 0..26 { parent.push(i); }
    step("init", "One slot per letter a … z. Twenty-six is the whole universe here, so the structure is O(1) space.",
        [ text("equations", eqs) ]);
    step("seed", "Every letter starts as its own component — nothing is known to be equal yet.",
        [ kv(global::ROOTS, roots_map(parent, letters)) ]);

    for k in 0..eqs.len() {
        let e = eqs[k];
        let op = e[1].to_string();
        set(#{ equation: S(e) });
        step("join", `Pass 1, equation ${k}: ${e}.`,
            [ text("equations", eqs), kv(global::ROOTS, roots_map(parent, letters)) ]);

        if op != "=" {
            step("iseq", `${e} is an inequality — skip it for now. Pass 1 only believes ==.`,
                [ kv(global::ROOTS, roots_map(parent, letters)) ]);
            continue;
        }

        let x = ord(e[0].to_string()) - ord("a");
        let y = ord(e[3].to_string()) - ord("a");
        step("iseq", `${e} says ${e[0]} and ${e[3]} must hold the same value — merge their components.`,
            [ kv(global::ROOTS, roots_map(parent, letters)) ]);

        step("roots", `Find the root of ${e[0]} and of ${e[3]}.`,
            [ kv(global::ROOTS, roots_map(parent, letters)) ], "call");
        let ra = find_root(parent, x);
        parent = ra[0];
        let rb = find_root(parent, y);
        parent = rb[0];

        parent[ra[1]] = rb[1];
        log(`${e[0]} and ${e[3]} are now one component`);
        step("union", `Point ${chr(ra[1] + ord("a"))} at ${chr(rb[1] + ord("a"))}. Every letter already equal to either one comes along.`,
            [ kv(global::ROOTS, roots_map(parent, letters)) ]);
    }

    for k in 0..eqs.len() {
        let e = eqs[k];
        let op = e[1].to_string();
        set(#{ equation: S(e) });
        step("check", `Pass 2, equation ${k}: ${e}.`,
            [ text("equations", eqs), kv(global::ROOTS, roots_map(parent, letters)) ]);

        if op != "!" { continue; }

        let x = ord(e[0].to_string()) - ord("a");
        let y = ord(e[3].to_string()) - ord("a");
        let ra = find_root(parent, x);
        parent = ra[0];
        let rb = find_root(parent, y);
        parent = rb[0];

        if ra[1] == rb[1] {
            log(`${e} contradicts the equalities`, "result");
            step("clash",
                `${e[0]} and ${e[3]} are both in component ${chr(ra[1] + ord("a"))}, so they are forced equal — and ${e} demands they differ.`,
                [ kv(global::ROOTS, roots_map(parent, letters)).bad([e[0].to_string(), e[3].to_string()]) ]);
            result("false");
            step("no", "No assignment can satisfy all of them: return false.",
                [ text("equations", eqs) ], "return");
            leave("false");
            return;
        }

        step("clash", `${e[0]} and ${e[3]} sit in different components, so they are free to differ. ${e} is satisfiable.`,
            [ kv(global::ROOTS, roots_map(parent, letters)).done([e[0].to_string(), e[3].to_string()]) ]);
    }

    log("all equations satisfiable", "result");
    result("true");
    step("yes", "Nothing contradicted: give each component its own number and every equation holds.",
        [ kv(global::ROOTS, roots_map(parent, letters)) ], "return");
    leave("true");
}
