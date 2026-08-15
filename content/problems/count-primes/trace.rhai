// Count Primes — let the primes find the composites.
//
// Trial division asks "does anything divide k?" once per k, repeating the same
// divisions endlessly. The sieve turns the question around: each prime walks
// through its own multiples and crosses them out, so a composite is *reached*
// rather than searched for. Starting at p·p is the observation that makes it
// cheap — any smaller multiple of p has a factor below p and was already
// crossed out by it.

fn marks(composite, n) {
    let out = [];
    for k in 0..n { if composite[k] { out.push(k); } }
    out
}

fn numbers(n) {
    let out = [];
    for k in 0..n { out.push(k); }
    out
}

fn trace(input) {
    let n = input.n;

    enter("countPrimes", #{ n: N(n) });

    if n < 3 {
        step("small", `There are no primes below ${n} — the smallest prime is 2.`,
            [ text("answer", ["0"]) ]);
        log("answer 0", "result");
        result("0");
        step("ret", "Return 0.", [ text("answer", ["0"]) ], "return");
        leave("0");
        return;
    }
    step("small", `${n} is large enough to contain primes.`, [ text("range", [`checking 2..${n - 1}`]) ]);

    let composite = [];
    for k in 0..n { composite.push(false); }
    step("init", "Everything starts un-crossed. A number is prime exactly when nothing crosses it out.",
        [ array("numbers", numbers(n)) ]);

    let count = 0;
    set(#{ count: N(count) });
    step("count", "Nothing counted yet.", [ array("numbers", numbers(n)) ]);

    let p = 2;
    while p * p < n {
        step("outer", `p = ${p}. Only p up to √${n} needs considering: a composite below ${n} must have a factor no larger than that.`,
            [ array("numbers", numbers(n)).ptr("p", p).bad(marks(composite, n)) ]);

        if composite[p] {
            step("skip", `${p} is already crossed out, so it is composite — its multiples are multiples of its factors too, and those have already been swept.`,
                [ array("numbers", numbers(n)).ptr("p", p).bad(marks(composite, n)) ]);
            p += 1;
            continue;
        }
        step("skip", `${p} survived, so it is prime.`,
            [ array("numbers", numbers(n)).ptr("p", p).hl([p]).bad(marks(composite, n)) ]);

        let m = p * p;
        while m < n {
            step("inner", `Next multiple of ${p} to cross out: ${m}. Starting from ${p}·${p} rather than 2·${p} is safe — every smaller multiple has a factor below ${p} and is already gone.`,
                [ array("numbers", numbers(n)).ptr("m", m).hl([p]).bad(marks(composite, n)) ]);
            composite[m] = true;
            log(`${m} = ${p} × ${m / p}`);
            step("mark", `Cross out ${m}.`,
                [ array("numbers", numbers(n)).ptr("m", m).bad(marks(composite, n)) ]);
            m += p;
        }

        p += 1;
    }

    let primes = [];
    step("tally", "Whatever is still un-crossed is prime.",
        [ array("numbers", numbers(n)).bad(marks(composite, n)) ]);

    for k in 2..n {
        if !composite[k] {
            primes.push(k);
            step("isPrime", `${k} was never crossed out.`,
                [ array("numbers", numbers(n)).ptr("k", k).hl(primes).bad(marks(composite, n)) ]);
            count += 1;
            set(#{ count: N(count) });
            step("bump", `That makes ${count}.`,
                [ array("primes found", primes) ]);
        }
    }

    log(`answer ${count}`, "result");
    result(`${count}`);
    step("ret", `${plural(count, "prime")} below ${n}: ${join(primes, ", ")}.`,
        [ array("primes found", primes) ], "return");
    leave(`${count}`);
}
