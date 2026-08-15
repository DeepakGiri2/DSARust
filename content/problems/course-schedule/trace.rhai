// Course Schedule — cycle detection with three colours.
//
// Drawn as a graph so the back edge is literally visible: the moment the DFS
// reaches a course that is still on the current path (amber), the path has
// closed into a loop and no ordering can exist.
//
// state: 0 = untouched, 1 = on the current path, 2 = fully explored and fine.

fn edges_of(spec, n) {
    let out = [];
    for pair in words(spec) {
        let parts = pair.split(",");
        if parts.len() == 2 {
            let a = num(parts[0]);
            let b = num(parts[1]);
            if a >= 0 && a < n && b >= 0 && b < n { out.push([a, b]); }
        }
    }
    out
}

fn nodes_of(n) {
    let out = [];
    for i in 0..n { out.push(#{ id: i, label: `${i}` }); }
    out
}

fn visiting_of(state) {
    let out = [];
    for i in 0..state.len() { if state[i] == 1 { out.push(i); } }
    out
}

fn done_of(state) {
    let out = [];
    for i in 0..state.len() { if state[i] == 2 { out.push(i); } }
    out
}

fn dfs(course, adj, state, nodes, edges, path) {
    enter(`dfs(${course})`, #{ course: N(course) });

    if state[course] == 1 {
        step("cycle",
            `Course ${course} is already on the path we are standing on — that is a back edge, i.e. a cycle.`,
            [ graph("courses", nodes, edges).cur(course).bad([course])
                .frontier(visiting_of(state)).done(done_of(state)).active(path) ]);
        log(`cycle detected at course ${course}`, "result");
        step("backedge", "Return false: a course that ultimately requires itself can never be taken.",
            [ graph("courses", nodes, edges).bad([course]).done(done_of(state)) ], "return");
        leave("false");
        return [state, false];
    }

    if state[course] == 2 {
        step("donecheck", `Course ${course} was fully explored earlier and led to no cycle.`,
            [ graph("courses", nodes, edges).cur(course).done(done_of(state)) ]);
        step("skip", "Return true immediately — re-exploring it could not tell us anything new.",
            [ graph("courses", nodes, edges).done(done_of(state)) ], "return");
        leave("true");
        return [state, true];
    }

    state[course] = 1;
    set(#{ state: A(state) });
    step("visiting", `Mark course ${course} as "on the current path" and explore what it depends on.`,
        [ graph("courses", nodes, edges).cur(course)
            .frontier(visiting_of(state)).done(done_of(state)).active(path) ]);

    for dep in adj[course] {
        step("deploop", `Course ${course} requires course ${dep} first.`,
            [ graph("courses", nodes, edges).cur(course)
                .frontier(visiting_of(state)).done(done_of(state)).active([[course, dep]]) ]);

        path.push([course, dep]);
        step("rec", `Descend into course ${dep}.`,
            [ graph("courses", nodes, edges).cur(dep)
                .frontier(visiting_of(state)).done(done_of(state)).active(path) ], "call");

        let out = dfs(dep, adj, state, nodes, edges, path);
        state = out[0];
        if !out[1] {
            step("propagate", `The search below course ${dep} found a cycle, so course ${course} fails too.`,
                [ graph("courses", nodes, edges).bad([course, dep]).done(done_of(state)) ], "return");
            leave("false");
            return [state, false];
        }
        path.pop();
    }

    state[course] = 2;
    set(#{ state: A(state) });
    log(`course ${course} is safe`);
    step("done", `Every prerequisite of course ${course} checked out — mark it finished.`,
        [ graph("courses", nodes, edges).cur(course)
            .frontier(visiting_of(state)).done(done_of(state)) ]);
    step("ok", `Return true for course ${course}.`,
        [ graph("courses", nodes, edges).done(done_of(state)) ], "return");
    leave("true");
    [state, true];
}

fn trace(input) {
    let n = input.numCourses;
    let edges = edges_of(input.prereqs, n);
    let nodes = nodes_of(n);

    let adj = [];
    for i in 0..n { adj.push([]); }
    let state = [];
    for i in 0..n { state.push(0); }

    enter("canFinish", #{ numCourses: N(n) });
    step("init", `${n} courses, no prerequisites wired up yet.`,
        [ graph("courses", nodes, []) ]);

    for e in edges {
        adj[e[0]].push(e[1]);
    }
    log(`${edges.len()} prerequisite edge(s)`);
    step("build",
        `Build the adjacency list: an arrow from a to b means "a requires b". An ordering exists exactly when these arrows contain no cycle.`,
        [ graph("courses", nodes, edges) ]);

    set(#{ state: A(state) });
    step("state", "Every course starts untouched. The colours are: untouched, on the current path, and finished.",
        [ graph("courses", nodes, edges) ]);

    for i in 0..n {
        step("mainloop", `Start a search from course ${i} (the graph may not be connected, so every course gets a turn).`,
            [ graph("courses", nodes, edges).cur(i).done(done_of(state)) ]);

        step("call", `Run the depth-first search from course ${i}.`,
            [ graph("courses", nodes, edges).cur(i).done(done_of(state)) ], "call");
        let out = dfs(i, adj, state, nodes, edges, []);
        state = out[0];

        if !out[1] {
            log("answer: false", "result");
            result("false");
            step("retfalse", "A cycle exists, so the courses cannot all be finished. Return false.",
                [ graph("courses", nodes, edges).bad(seq(0, n - 1)) ], "return");
            leave("false");
            return;
        }
    }

    log("answer: true", "result");
    result("true");
    step("rettrue", "Every course was explored without ever meeting a back edge — the prerequisites form a DAG, so a valid order exists.",
        [ graph("courses", nodes, edges).done(seq(0, n - 1)) ], "return");
    leave("true");
}
