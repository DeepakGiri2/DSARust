// Dota2 Senate — ban the next one to act, and nothing else.
//
// Optimal play here has a very short description: the dangerous opponent is
// the one who acts soonest, so ban that one. Everything else follows. Two
// queues of indices make "who acts soonest on each side" a single comparison,
// and adding n to a surviving senator's index is how they take their place in
// the next round without any explicit notion of rounds at all.

fn labels(q) {
    let out = [];
    for i in q { out.push(`${i}`); }
    out
}

fn trace(input) {
    let senate = input.senate;
    let cs = chars(senate);
    let n = cs.len();

    enter("predictPartyVictory", #{ n: N(n) });

    let radiant = [];
    let dire = [];
    step("init", "One queue of positions per party.",
        [ array("senate", cs), queue("Radiant (positions)", labels(radiant)),
          queue("Dire (positions)", labels(dire)) ]);

    set(#{ n: N(n) });
    step("n", `${n} senators, so adding ${n} to a position moves that senator into the next round.`,
        [ array("senate", cs) ]);

    for i in 0..n {
        if cs[i] == "R" { radiant.push(i); } else { dire.push(i); }
    }
    step("fill", `Radiant holds ${plural(radiant.len(), "senator")}, Dire ${plural(dire.len(), "senator")}.`,
        [ array("senate", cs), queue("Radiant (positions)", labels(radiant)),
          queue("Dire (positions)", labels(dire)) ]);

    while radiant.len() > 0 && dire.len() > 0 {
        step("loop", "Both parties still have senators, so another exchange happens.",
            [ queue("Radiant (positions)", labels(radiant)),
              queue("Dire (positions)", labels(dire)) ]);

        let r = radiant[0];
        radiant.remove(0);
        step("popR", `Radiant's next senator acts at position ${r}.`,
            [ queue("Radiant (positions)", labels(radiant)),
              queue("Dire (positions)", labels(dire)) ]);

        let d = dire[0];
        dire.remove(0);
        step("popD", `Dire's next acts at position ${d}.`,
            [ queue("Radiant (positions)", labels(radiant)),
              queue("Dire (positions)", labels(dire)) ]);

        if r < d {
            step("earlier", `${r} comes before ${d}, so the Radiant senator acts first and bans the Dire one. Banning the *next* opponent is optimal: any other choice leaves a nearer threat alive.`,
                [ queue("Radiant (positions)", labels(radiant)),
                  queue("Dire (positions)", labels(dire)) ]);
            radiant.push(r + n);
            log(`Radiant ${r} bans Dire ${d}`);
            step("requeueR", `The survivor rejoins at ${r + n} — the same slot, one round later.`,
                [ queue("Radiant (positions)", labels(radiant)) ]);
        } else {
            step("later", `${d} comes before ${r}, so the Dire senator acts first.`,
                [ queue("Radiant (positions)", labels(radiant)),
                  queue("Dire (positions)", labels(dire)) ]);
            dire.push(d + n);
            log(`Dire ${d} bans Radiant ${r}`);
            step("requeueD", `The survivor rejoins at ${d + n}.`,
                [ queue("Dire (positions)", labels(dire)) ]);
        }
    }

    let winner = "";
    if radiant.len() > 0 {
        winner = "Radiant";
        log("Radiant wins", "result");
        step("check", "Dire's queue is empty — every Dire senator has been banned.",
            [ queue("Radiant (positions)", labels(radiant)),
              queue("Dire (positions)", labels(dire)) ]);
        step("retR", "Radiant wins.", [ text("answer", ["Radiant"]) ], "return");
    } else {
        winner = "Dire";
        log("Dire wins", "result");
        step("check", "Radiant's queue is empty.",
            [ queue("Radiant (positions)", labels(radiant)),
              queue("Dire (positions)", labels(dire)) ]);
        step("retD", "Dire wins.", [ text("answer", ["Dire"]) ], "return");
    }

    result(winner);
    leave(winner);
}
