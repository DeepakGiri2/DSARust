// Burst Balloons with arrows — sort by end, shoot at the end.
//
// The exchange argument: consider the balloon that finishes first. Some arrow
// must burst it, and moving that arrow to its exact end point can only add
// balloons to what it hits, never remove any. So shooting at the earliest end
// is always at least as good as any alternative — which is what makes the
// greedy choice safe, and why sorting by start is the version that fails.

fn parse_points(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

fn lines_of(points, upto, end, arrows) {
    let out = [];
    for i in 0..points.len() {
        let mark = if i < upto { "burst" } else { "     " };
        out.push(`${mark}  [${points[i][0]}, ${points[i][1]}]`);
    }
    out.push(`arrow at x = ${end}, arrows used = ${arrows}`);
    out
}

fn sort_by_end(points) {
    let out = [];
    for p in points {
        let i = 0;
        while i < out.len() && out[i][1] <= p[1] { i += 1; }
        out.insert(i, p);
    }
    out
}

fn ends_of(points) {
    let out = [];
    for p in points { out.push(p[1]); }
    out
}

fn starts_of(points) {
    let out = [];
    for p in points { out.push(p[0]); }
    out
}

fn trace(input) {
    let raw = parse_points(input.points);

    enter("findMinArrowShots", #{ balloons: N(raw.len()) });

    step("sort", "Sort by *end* point. The balloon that finishes first is the one that constrains the first arrow.",
        [ array("ends, unsorted", ends_of(raw)) ]);
    let points = sort_by_end(raw);
    step("sort", `Sorted: ${brackets(ends_of(points))}.`,
        [ array("starts", starts_of(points)), array("ends", ends_of(points)) ]);

    let arrows = 1;
    set(#{ arrows: N(1) });
    step("init", "At least one arrow is needed, so start the count at 1.",
        [ text("balloons", lines_of(points, 0, points[0][1], arrows)) ]);

    let end = points[0][1];
    set(#{ end: N(end) });
    step("first", `Shoot the first arrow at x = ${end}, the earliest end point. Anything that started at or before ${end} pops with it.`,
        [ array("ends", ends_of(points)).hl([0]),
          text("balloons", lines_of(points, 1, end, arrows)) ]);

    let burst = 1;
    for i in 1..points.len() {
        set(#{ i: N(i), start: N(points[i][0]) });
        step("loop", `Balloon ${i} spans [${points[i][0]}, ${points[i][1]}].`,
            [ array("starts", starts_of(points)).ptr("i", i).hl([i]),
              array("ends", ends_of(points)).ptr("i", i),
              text("balloons", lines_of(points, burst, end, arrows)) ]);

        if points[i][0] > end {
            step("miss", `It starts at ${points[i][0]}, after the arrow at ${end} — this one survives.`,
                [ array("starts", starts_of(points)).bad([i]),
                  text("balloons", lines_of(points, burst, end, arrows)) ]);
            arrows += 1;
            set(#{ arrows: N(arrows) });
            step("shoot", `So a new arrow is needed: ${arrows} so far.`,
                [ text("balloons", lines_of(points, burst, end, arrows)) ]);
            end = points[i][1];
            burst = i + 1;
            set(#{ end: N(end) });
            step("newend", `Aim it at ${end}, this balloon's end — again the earliest end among those still up.`,
                [ array("ends", ends_of(points)).hl([i]),
                  text("balloons", lines_of(points, burst, end, arrows)) ]);
        } else {
            burst = i + 1;
            step("miss", `It starts at ${points[i][0]} ≤ ${end}, so the arrow already in the air pops it too. No new arrow.`,
                [ array("starts", starts_of(points)).hl([i]),
                  text("balloons", lines_of(points, burst, end, arrows)) ]);
        }
    }

    log(`${plural(arrows, "arrow")}`, "result");
    result(`${arrows}`);
    step("ret", `${plural(arrows, "arrow")} bursts all ${points.len()}.`,
        [ text("balloons", lines_of(points, points.len(), end, arrows)) ], "return");
    leave(`${arrows}`);
}
