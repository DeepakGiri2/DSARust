// Minimum Cost For Tickets — index the calendar, not the trips.
//
// Indexing travel days makes a 7-day pass awkward: how many trips does it
// cover? Indexing calendar days makes it trivial — the pass covers the window
// [d-6..d] whatever is in it, so its cost is its price plus dp from just
// before the window. Non-travel days exist only to carry the previous answer
// forward, and that carrying is what makes those window lookups meaningful.

fn validate(input) {
    if input.days.len() == 0 {
        return "there must be at least one travel day";
    }
    for i in 1..input.days.len() {
        if input.days[i] <= input.days[i - 1] {
            return "travel days must be strictly increasing";
        }
    }
    if input.costs.len() != 3 {
        return "costs must hold exactly three prices: 1-day, 7-day, 30-day";
    }
}

fn min_of3(a, b, c) {
    let m = if a < b { a } else { b };
    if c < m { c } else { m }
}

fn travel_marks(travel) {
    let out = [];
    for d in 0..travel.len() { if travel[d] { out.push(d); } }
    out
}

fn trace(input) {
    let days = input.days;
    let costs = input.costs;
    let last = days[days.len() - 1];

    enter("mincostTickets", #{ last: N(last) });
    step("span", `The last travel day is ${last}, so nothing beyond it matters.`,
        [ array("travel days", days), array("pass prices (1, 7, 30 day)", costs) ]);

    let travel = [];
    for d in 0..(last + 1) { travel.push(false); }
    step("mark", "Turn the list of trips into a per-day flag, so the calendar can be walked directly.",
        [ array("travel days", days) ]);

    for d in days { travel[d] = true; }
    step("markLoop", `${plural(days.len(), "travel day")} marked on a calendar of ${last} days.`,
        [ array("day", seq(0, last)).hl(travel_marks(travel)) ]);

    let dp = [];
    for d in 0..(last + 1) { dp.push(0); }
    step("init", "dp[d] is the cheapest way to have every travel day up to d covered. Day 0 costs nothing.",
        [ array("dp (cheapest through day d)", dp) ]);

    for d in 1..(last + 1) {
        step("loop", `Day ${d}.`,
            [ array("dp (cheapest through day d)", dp).ptr("d", d).hl(travel_marks(travel))
                .done(before(d)) ]);

        if !travel[d] {
            step("rest", `No travel on day ${d}, so buying anything today would be wasted.`,
                [ array("dp (cheapest through day d)", dp).ptr("d", d) ]);
            dp[d] = dp[d - 1];
            step("carry", `Carry yesterday's cost forward: ${dp[d]}. This is what lets a pass bought later look back into a rest day and still find a meaningful number.`,
                [ array("dp (cheapest through day d)", dp).ptr("d", d).hl([d]) ]);
            continue;
        }

        let from1 = if d - 1 > 0 { d - 1 } else { 0 };
        let one = dp[from1] + costs[0];
        step("one", `A 1-day pass covers only today: ${costs[0]} on top of dp[${from1}] = ${dp[from1]}, so ${one}.`,
            [ array("dp (cheapest through day d)", dp).pointers(#{ from: from1, d: d }).hl([from1]) ]);

        let from7 = if d - 7 > 0 { d - 7 } else { 0 };
        let seven = dp[from7] + costs[1];
        step("seven", `A 7-day pass covers days ${d - 6}..${d}, so it starts from dp[${from7}] = ${dp[from7]}: ${seven}.`,
            [ array("dp (cheapest through day d)", dp).pointers(#{ from: from7, d: d })
                .hl(span(from7, d)) ]);

        let from30 = if d - 30 > 0 { d - 30 } else { 0 };
        let thirty = dp[from30] + costs[2];
        step("thirty", `A 30-day pass reaches back to dp[${from30}] = ${dp[from30]}: ${thirty}.`,
            [ array("dp (cheapest through day d)", dp).pointers(#{ from: from30, d: d })
                .hl(span(from30, d)) ]);

        dp[d] = min_of3(one, seven, thirty);
        log(`day ${d}: cheapest ${dp[d]}`);
        step("best", `Cheapest of ${one}, ${seven} and ${thirty} is ${dp[d]}.`,
            [ array("dp (cheapest through day d)", dp).ptr("d", d).hl([d]),
              text("options", [`1-day: ${one}`, `7-day: ${seven}`, `30-day: ${thirty}`]) ]);
    }

    log(`answer ${dp[last]}`, "result");
    result(`${dp[last]}`);
    step("ret", `Covering every travel day costs ${dp[last]}.`,
        [ bars("dp (cheapest through day d)", dp).ptr("last", last).hl([last]) ], "return");
    leave(`${dp[last]}`);
}
