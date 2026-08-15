// Number of Provinces — the component counter is the answer.
//
// The insight worth watching: nothing ever counts components at the end. The
// count starts at n (everybody alone) and drops by one on each *successful*
// merge. An edge between two cities already in the same province changes
// nothing — which is exactly what the roots tell you, in one comparison.

const PARENT = "parent of each city";

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

fn city_labels(n) {
    let out = [];
    for i in 0..n { out.push(`${i}`); }
    out
}

fn edges_upto(m, n, i, j) {
    // Every 1 in the upper triangle at or before (i, j), as [from, to].
    let out = [];
    for r in 0..n {
        for c in (r + 1)..n {
            if r < i || (r == i && c <= j) {
                if num(m[r][c]) == 1 { out.push([r, c]); }
            }
        }
    }
    out
}

fn trace(input) {
    let m = grid_rows(input.isConnected);
    let n = m.len();

    enter("findCircleNum", #{ cities: N(n) });

    let parent = [];
    for i in 0..n { parent.push(i); }
    step("init", `${n} cities. The parent array is the whole data structure — no adjacency lists, no visited set.`,
        [ graph("cities", city_labels(n), []), array(global::PARENT, parent) ]);
    step("seed", "Every city is its own parent: n components, none of them joined yet.",
        [ array(global::PARENT, parent) ]);

    let count = n;
    set(#{ count: N(count) });
    step("count", `count = ${n}. From here it only ever goes down, and only when a merge actually happens.`,
        [ array(global::PARENT, parent), text("components", [`count = ${count}`]) ]);

    for i in 0..n {
        step("row", `City ${i}: look at the cities after it. The lower triangle is the same information mirrored, so skipping it costs nothing.`,
            [ grid("isConnected", m).cur([i, i]),
              array(global::PARENT, parent) ]);

        for j in (i + 1)..n {
            step("col", `Is city ${i} directly connected to city ${j}?`,
                [ grid("isConnected", m).cur([i, j]),
                  graph("cities", city_labels(n), edges_upto(m, n, i, j - 1)).frontier([i, j]) ]);

            if num(m[i][j]) != 1 {
                step("edge", `isConnected[${i}][${j}] = 0 — no direct road. Nothing to merge.`,
                    [ grid("isConnected", m).cur([i, j]),
                      array(global::PARENT, parent) ]);
                continue;
            }

            step("edge", `isConnected[${i}][${j}] = 1 — a road. But it only matters if it joins two *different* provinces.`,
                [ grid("isConnected", m).cur([i, j]),
                  graph("cities", city_labels(n), edges_upto(m, n, i, j)).frontier([i, j]) ]);

            step("find", `Find the root of ${i} and the root of ${j}.`,
                [ array(global::PARENT, parent) ], "call");
            let ra = find_root(parent, i);
            parent = ra[0];
            let rb = find_root(parent, j);
            parent = rb[0];
            set(#{ a: N(ra[1]), b: N(rb[1]) });

            if ra[1] == rb[1] {
                step("same", `Both answer ${ra[1]}: cities ${i} and ${j} were already in the same province. The road is redundant, count is untouched.`,
                    [ graph("cities", city_labels(n), edges_upto(m, n, i, j)).done([i, j]),
                      text("components", [`count = ${count}`, "no change"]) ]);
                continue;
            }

            step("same", `Roots ${ra[1]} and ${rb[1]} differ — two separate provinces are about to become one.`,
                [ array(global::PARENT, parent).hl([ra[1], rb[1]]),
                  text("components", [`count = ${count}`]) ]);

            parent[ra[1]] = rb[1];
            log(`merge ${ra[1]} into ${rb[1]}`);
            step("union", `Hang ${ra[1]}'s tree under ${rb[1]}. One write joins two provinces of any size.`,
                [ array(global::PARENT, parent).hl([ra[1], rb[1]]),
                  graph("cities", city_labels(n), edges_upto(m, n, i, j)).done([i, j]) ]);

            count -= 1;
            set(#{ count: N(count) });
            step("shrink", `One fewer province: count = ${count}.`,
                [ array(global::PARENT, parent), text("components", [`count = ${count}`]) ]);
        }
    }

    log(`${plural(count, "province")}`, "result");
    result(`${count}`);
    step("ret", `${plural(count, "province")}. The counter was maintained all along — no final sweep needed.`,
        [ graph("cities", city_labels(n), edges_upto(m, n, n, n)),
          array(global::PARENT, parent),
          text("components", [`count = ${count}`]) ],
        "return");
    leave(`${count}`);
}
