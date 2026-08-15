// Asteroid Collision — only one arrangement can ever collide.
//
// A left-mover behind a left-mover never meets it; two right-movers never meet
// either. The single dangerous pattern is a right-mover already standing when
// a left-mover arrives — and a stack of survivors puts exactly that asteroid
// on top. So the entire problem becomes: fight the top until something stops.

fn labels(arr) {
    let out = [];
    for a in arr {
        out.push(if a > 0 { `${a}>` } else { `<${-a}` });
    }
    out
}

fn trace(input) {
    let ast = input.asteroids;
    let n = ast.len();
    let stack = [];

    enter("asteroidCollision", #{});
    step("init", "Survivors so far. A right-mover on top is the only thing a new left-mover can hit.",
        [ array("asteroids", labels(ast)), stack("survivors", labels(stack)) ]);

    for i in 0..n {
        let a = ast[i];
        step("loop", `Asteroid ${i}: ${a} — size ${if a < 0 { -a } else { a }}, moving ${if a > 0 { "right" } else { "left" }}.`,
            [ array("asteroids", labels(ast)).ptr("i", i).done(before(i)),
              stack("survivors", labels(stack)) ]);

        let alive = true;
        step("alive", "Assume it survives until something proves otherwise.",
            [ array("asteroids", labels(ast)).ptr("i", i).hl([i]),
              stack("survivors", labels(stack)) ]);

        while alive && a < 0 && stack.len() > 0 && stack[stack.len() - 1] > 0 {
            step("collide", `It moves left and the survivor on top moves right, so they must meet.`,
                [ array("asteroids", labels(ast)).ptr("i", i).hl([i]),
                  stack("survivors", labels(stack)) ]);

            let top = stack[stack.len() - 1];
            step("peek", `The standing asteroid is size ${top}; the newcomer is size ${-a}.`,
                [ array("asteroids", labels(ast)).ptr("i", i),
                  stack("survivors", labels(stack)) ]);

            if top < -a {
                step("smaller", `${top} < ${-a}, so the standing one explodes — and the newcomer carries on, possibly to hit the next one.`,
                    [ array("asteroids", labels(ast)).ptr("i", i),
                      stack("survivors", labels(stack)) ]);
                stack.truncate(stack.len() - 1);
                log(`${-a} destroys ${top}`);
                step("popTop", "Remove it and go round again.",
                    [ array("asteroids", labels(ast)).ptr("i", i),
                      stack("survivors", labels(stack)).popped(`${top}>`) ]);
            } else if top == -a {
                step("equal", `Equal sizes: ${top} against ${-a}. Both are destroyed.`,
                    [ array("asteroids", labels(ast)).ptr("i", i).bad([i]),
                      stack("survivors", labels(stack)) ]);
                stack.truncate(stack.len() - 1);
                alive = false;
                log(`${top} and ${-a} annihilate`);
                step("popBoth", "Remove the standing one, and the newcomer is gone too.",
                    [ array("asteroids", labels(ast)).ptr("i", i).bad([i]),
                      stack("survivors", labels(stack)).popped(`${top}>`) ]);
            } else {
                step("bigger", `${top} > ${-a}, so the standing asteroid wins.`,
                    [ array("asteroids", labels(ast)).ptr("i", i).bad([i]),
                      stack("survivors", labels(stack)) ]);
                alive = false;
                log(`${top} destroys ${-a}`);
                step("die", "The newcomer explodes and the loop ends.",
                    [ array("asteroids", labels(ast)).ptr("i", i).bad([i]),
                      stack("survivors", labels(stack)) ]);
            }
        }

        if alive {
            step("survives", "Nothing destroyed it.",
                [ array("asteroids", labels(ast)).ptr("i", i),
                  stack("survivors", labels(stack)) ]);
            stack.push(a);
            set(#{ survivors: N(stack.len()) });
            step("push", `Add it to the survivors.`,
                [ array("asteroids", labels(ast)).ptr("i", i).done(before(i + 1)),
                  stack("survivors", labels(stack)).pushed(true) ]);
        }
    }

    let answer = join(stack, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(stack.len(), "asteroid")} survive: ${answer}. Each one was pushed at most once and popped at most once.`,
        [ array("survivors", labels(stack)).done(seq(0, stack.len() - 1)) ], "return");
    leave(answer);
}
