// Boats to Save People — greedy pairing of the lightest with the heaviest.
//
// The heaviest person needs a boat regardless. The only question is whether
// anyone can ride with them, and if anyone can, the lightest person can — so
// pairing those two is never worse than any other choice.

fn trace(input) {
    let people = input.people;
    let limit = input.limit;
    let n = people.len();

    for i in 1..n {
        let j = i;
        while j > 0 && people[j - 1] > people[j] {
            swap_at(people, j - 1, j);
            j -= 1;
        }
    }

    enter("numRescueBoats", #{ limit: N(limit) });
    step("sort", `Sort by weight: ${brackets(people)}. The greedy choice only makes sense on sorted people.`,
        [ bars(`people   (limit = ${limit})`, people) ]);

    let l = 0;
    let r = n - 1;
    set(#{ l: N(l), r: N(r) });
    step("init", "The lightest person and the heaviest person.",
        [ bars(`people   (limit = ${limit})`, people).pointers(#{ l: l, r: r }) ]);

    let boats = 0;
    set(#{ boats: N(boats) });
    step("boats", "No boats used yet.",
        [ bars(`people   (limit = ${limit})`, people).pointers(#{ l: l, r: r }) ]);

    while l <= r {
        step("loop", `${r - l + 1} person(s) still waiting.`,
            [ bars(`people   (limit = ${limit})`, people).pointers(#{ l: l, r: r })
                .window(l, r).done(dead(n, l, r)) ]);

        if people[l] + people[r] <= limit {
            log(`${people[l]} + ${people[r]} <= ${limit} — they share`);
            step("fit", `${people[l]} + ${people[r]} = ${people[l] + people[r]} fits in one boat, so the lightest rides with the heaviest.`,
                [ bars(`people   (limit = ${limit})`, people).pointers(#{ l: l, r: r })
                    .hl([l, r]).done(dead(n, l, r)) ]);
            l += 1;
            set(#{ l: N(l) });
            step("moveL", `The lightest is on board: l = ${l}.`,
                [ bars(`people   (limit = ${limit})`, people).pointers(#{ l: l, r: r })
                    .window(l, r).done(dead(n, l, r)) ]);
        } else {
            step("fit", `${people[l]} + ${people[r]} = ${people[l] + people[r]} exceeds ${limit}. If even the lightest cannot join, nobody can — the heaviest sails alone.`,
                [ bars(`people   (limit = ${limit})`, people).pointers(#{ l: l, r: r })
                    .hl([r]).bad([l]).done(dead(n, l, r)) ]);
        }

        r -= 1;
        set(#{ r: N(r) });
        step("moveR", `The heaviest is always on this boat: r = ${r}.`,
            [ bars(`people   (limit = ${limit})`, people).pointers(#{ l: l, r: r })
                .window(l, r).done(dead(n, l, r)) ]);

        boats += 1;
        set(#{ boats: N(boats) });
        step("count", `That is ${plural(boats, "boat")} so far.`,
            [ bars(`people   (limit = ${limit})`, people).pointers(#{ l: l, r: r })
                .window(l, r).done(dead(n, l, r)) ]);
    }

    log(`answer ${boats}`, "result");
    result(`${boats}`);
    step("ret", `Everyone is aboard: ${plural(boats, "boat")}.`,
        [ bars(`people   (limit = ${limit})`, people).done(seq(0, n - 1)) ], "return");
    leave(`${boats}`);
}
