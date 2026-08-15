// Daily Temperatures — a monotonic stack of days still waiting for an answer.
//
// The stack only ever holds indices whose temperatures decrease downward. That
// invariant is why a single warmer day can settle several waiting days at once
// and why the whole scan is linear.

// Rhai functions live at the top level only.
fn stack_values(stack, temps) {
    let out = [];
    for i in stack { out.push(`${temps[i]}@${i}`); }
    out
}

fn trace(input) {
    let temps = input.temps;
    let n = temps.len();
    let answer = [];
    for i in 0..n { answer.push(0); }
    let stack = [];

    enter("dailyTemperatures", #{ temps: A(temps) });
    set(#{ answer: A(answer) });
    step("init", "The answer starts as all zeros: a day with no warmer day ahead keeps 0.",
        [ array("temps", temps), array("answer", answer) ]);

    set(#{ stack: A(stack) });
    step("initStack",
        "The stack holds days that are still waiting for a warmer day. It stays decreasing from bottom to top.",
        [ array("temps", temps), array("answer", answer),
          stack("waiting (temp@day)", stack_values(stack, temps)) ]);

    for i in 0..n {
        set(#{ i: N(i) });
        step("loop", `Day ${i}: ${temps[i]} degrees.`,
            [ array("temps", temps).ptr("i", i).done(before(i)), array("answer", answer),
              stack("waiting (temp@day)", stack_values(stack, temps)) ]);

        while stack.len() > 0 && temps[i] > temps[stack[stack.len() - 1]] {
            let j = stack[stack.len() - 1];
            step("while", `${temps[i]} is warmer than day ${j}'s ${temps[j]}, so day ${j} has its answer.`,
                [ array("temps", temps).ptr("i", i).hl([i, j]).done(before(i)),
                  array("answer", answer),
                  stack("waiting (temp@day)", stack_values(stack, temps)) ]);
            step("peek", `The top of the stack is day ${j}.`,
                [ array("temps", temps).ptr("i", i).hl([j]).done(before(i)),
                  array("answer", answer),
                  stack("waiting (temp@day)", stack_values(stack, temps)) ]);

            stack.pop();
            step("pop", `Day ${j} is settled — take it off the stack.`,
                [ array("temps", temps).ptr("i", i).done(before(i)), array("answer", answer),
                  stack("waiting (temp@day)", stack_values(stack, temps)).popped(`${temps[j]}@${j}`) ]);

            answer[j] = i - j;
            set(#{ answer: A(answer) });
            log(`answer[${j}] = ${i} - ${j} = ${i - j}`);
            step("set", `Day ${j} waited ${i - j} day(s) for a warmer one.`,
                [ array("temps", temps).ptr("i", i).done(before(i)),
                  array("answer", answer).hl([j]),
                  stack("waiting (temp@day)", stack_values(stack, temps)) ]);
        }

        stack.push(i);
        set(#{ stack: A(stack) });
        step("push",
            `Day ${i} has no warmer day yet, so it joins the waiting stack. Everything below it is warmer, which keeps the stack decreasing.`,
            [ array("temps", temps).ptr("i", i).done(before(i)), array("answer", answer),
              stack("waiting (temp@day)", stack_values(stack, temps)).pushed(true) ]);
    }

    log(`${stack.len()} day(s) never saw a warmer day`, "result");
    result(brackets(answer));
    step("ret",
        `Whatever is left on the stack never saw a warmer day, and those entries keep their 0. Answer: ${brackets(answer)}.`,
        [ array("temps", temps).done(seq(0, n - 1)), array("answer", answer),
          stack("waiting (temp@day)", stack_values(stack, temps)) ],
        "return");
    leave(brackets(answer));
}
