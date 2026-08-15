// Eliminate Maximum Monsters — shoot the soonest arrival, always.
//
// Suppose an optimal plan shoots a later-arriving monster before an earlier
// one. Swapping those two shots cannot break anything: the earlier one is now
// shot sooner, and the later one still has more time than the earlier one did.
// So there is always an optimal plan in arrival order — which makes sorting
// the whole algorithm. Comparing arrival times as a cross-multiplication keeps
// it exact, with no division anywhere.

fn labels(dist, speed, order) {
    let out = [];
    for i in order { out.push(`#${i}: ${dist[i]}/${speed[i]}`); }
    out
}

fn trace(input) {
    let dist = input.dist;
    let speed = input.speed;
    let n = dist.len();

    enter("eliminateMaximum", #{ n: N(n) });

    let idx = [];
    for i in 0..n { idx.push(i); }
    step("order", "Monster i arrives after dist[i] / speed[i] minutes.",
        [ array("distance", dist), array("speed", speed) ]);

    for i in 1..n {
        let j = i;
        while j > 0 && dist[idx[j - 1]] * speed[idx[j]] > dist[idx[j]] * speed[idx[j - 1]] {
            swap_at(idx, j - 1, j);
            j -= 1;
        }
    }
    step("sort", "Sorted by arrival time. Comparing a/b against c/d as a·d against c·b avoids division entirely, so there is no rounding to get wrong.",
        [ array("monsters, by arrival time", labels(dist, speed, idx)) ]);

    let killed = n;
    for i in 0..n {
        step("loop", `Minute ${i}: one shot is available.`,
            [ array("monsters, by arrival time", labels(dist, speed, idx))
                .ptr("i", i).done(before(i)) ]);

        let m = idx[i];
        step("next", `The soonest remaining arrival is monster #${m}, which covers ${dist[m]} at ${speed[m]} per minute.`,
            [ array("monsters, by arrival time", labels(dist, speed, idx)).ptr("i", i).hl([i]) ]);

        if dist[m] <= i * speed[m] {
            killed = i;
            log(`monster #${m} arrives by minute ${i}`, "result");
            step("late", `By minute ${i} it has already travelled ${i * speed[m]}, which is at least its distance ${dist[m]} — it has arrived, and the game is over.`,
                [ array("monsters, by arrival time", labels(dist, speed, idx)).bad([i]),
                  text("timing", [`minute: ${i}`, `distance covered: ${i * speed[m]}`,
                                  `distance to city: ${dist[m]}`]) ]);
            step("ret", `${plural(i, "monster")} eliminated.`,
                [ array("monsters, by arrival time", labels(dist, speed, idx)).done(before(i)) ],
                "return");
            break;
        }

        step("late", `At minute ${i} it has covered only ${i * speed[m]} of ${dist[m]}, so there is still time to shoot it.`,
            [ array("monsters, by arrival time", labels(dist, speed, idx)).ptr("i", i).hl([i]),
              text("timing", [`minute: ${i}`, `distance covered: ${i * speed[m]}`,
                              `distance to city: ${dist[m]}`]) ]);
    }

    if killed == n {
        log(`all ${n} eliminated`, "result");
        step("all", `Every monster was shot before arriving: ${n} in total.`,
            [ array("monsters, by arrival time", labels(dist, speed, idx)).done(seq(0, n - 1)) ],
            "return");
    }

    result(`${killed}`);
    leave(`${killed}`);
}
