// Car Fleet — process from the front of the road backwards.
//
// A car can only ever be blocked by a car *ahead* of it, so taking them in
// order of position (closest to the target first) means each car is compared
// with a fleet that is already final. Arrival time, not distance, is what
// decides: a car that would arrive no later than the fleet in front is
// absorbed into it.

// Rhai functions must live at the top level.
fn labels(idx, pos, spd) {
    let out = [];
    for i in idx { out.push(`p${pos[i]}/s${spd[i]}`); }
    out
}

fn trace(input) {
    let target = input.target;
    let pos = input.position;
    let spd = input.speed;
    let n = pos.len();

    // Indices sorted by position, closest to the target first.
    let idx = [];
    for i in 0..n { idx.push(i); }
    for i in 1..n {
        let j = i;
        while j > 0 && pos[idx[j - 1]] < pos[idx[j]] {
            swap_at(idx, j - 1, j);
            j -= 1;
        }
    }

    enter("carFleet", #{ target: N(target) });
    step("order", "Cars, by starting position.",
        [ bars(`position   (target = ${target})`, pos), array("speed", spd) ]);
    step("sort",
        "Sort so the car nearest the target comes first. A car can only be blocked by one ahead of it, so this order means every comparison is against a finished fleet.",
        [ array("cars, nearest the target first", labels(idx, pos, spd)) ]);

    // Two parallel stacks: `times` is what the comparison uses (scaled to stay
    // integral), `shown` is what the viewer reads.
    let times = [];
    let shown = [];
    set(#{ fleets: N(0) });
    step("init", "The stack holds one arrival time per fleet.",
        [ array("cars, nearest the target first", labels(idx, pos, spd)),
          stack("fleet arrival times", shown) ]);

    for k in 0..n {
        let i = idx[k];
        step("loop", `Car at position ${pos[i]} travelling at ${spd[i]}.`,
            [ array("cars, nearest the target first", labels(idx, pos, spd))
                .ptr("k", k).done(before(k)),
              stack("fleet arrival times", shown) ]);

        let dist = target - pos[i];
        let t = dist * 1000 / spd[i];
        set(#{ time: S(`${dist}/${spd[i]}`) });
        step("time", `It has ${dist} to travel, so it arrives at time ${dist}/${spd[i]}.`,
            [ array("cars, nearest the target first", labels(idx, pos, spd))
                .ptr("k", k).hl([k]),
              stack("fleet arrival times", shown) ]);

        if times.len() == 0 || t > times[times.len() - 1] {
            step("check",
                if times.len() == 0 {
                    "Nothing ahead of it — it leads its own fleet.".to_string()
                } else {
                    `It arrives later than the fleet ahead, so it can never catch up and stays separate.`
                },
                [ array("cars, nearest the target first", labels(idx, pos, spd)).ptr("k", k),
                  stack("fleet arrival times", shown) ]);
            times.push(t);
            shown.push(`${dist}/${spd[i]}`);
            set(#{ fleets: N(times.len()) });
            log(`new fleet, arrival ${dist}/${spd[i]}`);
            step("push", `That makes ${plural(times.len(), "fleet")}.`,
                [ array("cars, nearest the target first", labels(idx, pos, spd)).ptr("k", k),
                  stack("fleet arrival times", shown).pushed(true) ]);
        } else {
            step("check",
                `It would arrive no later than the fleet ahead, so it catches up and is absorbed — the count does not change.`,
                [ array("cars, nearest the target first", labels(idx, pos, spd))
                    .ptr("k", k).bad([k]),
                  stack("fleet arrival times", shown) ]);
        }
    }

    log(`answer ${times.len()}`, "result");
    result(`${times.len()}`);
    step("ret", `${plural(times.len(), "fleet")} reach the target.`,
        [ stack("fleet arrival times", shown) ], "return");
    leave(`${times.len()}`);
}
