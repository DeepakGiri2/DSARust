# dsa-runner

Sandboxed compile-and-run for **DSA Visualized**. Users type Go, C++, Java or
Python in the browser; the API assembles a complete program and sends it here;
this service compiles it once and runs it against each test case, then returns
stdout/stderr and a status per case. It never sees the expected answers — the
API compares outputs itself — so the only thing worth protecting is *the runner
and the next user's job from the program running right now*.

One binary serves two transports, chosen at startup:

- **http** (docker-compose, ECS): `axum` on `RUNNER_BIND` — `POST /v1/execute`,
  `GET /healthz`.
- **lambda** (AWS): selected when `AWS_LAMBDA_RUNTIME_API` is set; one job per
  invocation, no network route anywhere.

The wire contract is [`dsa-protocol`](../protocol); the environment variables and
ports are in [`docs/platform/SERVICES.md`](../../../docs/platform/SERVICES.md).

## Threat model

Every byte of the submitted program is hostile, and so is any file, socket or
process it creates. A submission may try to:

1. reach the network (exfiltrate, attack, or just phone home);
2. read the runner's secrets or memory, or another job's data;
3. escape its directory and read or write the host filesystem;
4. survive its job — a daemon that runs into the *next* user's job;
5. exhaust a resource: CPU, memory, disk, processes, output;
6. gain privileges (setuid binaries, ptrace, namespaces, mounts);
7. poison a shared cache so the next user runs tampered code.

What is **not** in scope: the confidentiality of the program's own output (the
API owns that), kernel 0-days (a microVM or the host kernel is the backstop),
and — for a namespace-less container — the world-writable `/tmp` (mitigated by
sweeping, see [Known limitations](#known-limitations)).

## Defense in depth

No single mechanism is trusted. From the outside in:

### 0. Deployment (outside this crate, assumed)

- **compose/ECS**: a container that starts as root but with every capability
  dropped except the six the runner needs to give jobs their own uid
  (`CHOWN`, `DAC_OVERRIDE`, `FOWNER`, `SETUID`, `SETGID`, `KILL`), with
  `no-new-privileges`, a read-only root filesystem and a size-capped `tmpfs`
  on `/tmp` (see `docker-compose.yml`). No `SYS_ADMIN` or `NET_ADMIN`, so no
  namespaces, cgroups or iptables are available to *us* — and none to the
  job either. The task's security group is what actually blocks the network
  at the edge.
- **Lambda**: a Firecracker microVM in isolated subnets with a security group
  that allows **no egress at all**; read-only root filesystem except `/tmp`.
  The microVM is the strong boundary; everything below is defence in depth
  within it.

### 1. Runner process hardening (`hardening.rs`, at startup)

- `PR_SET_DUMPABLE = 0`: the runner's `/proc/<pid>/{environ,mem,maps}` become
  root-only and it cannot be `ptrace`d, even by a process of its own uid (which
  is the case in Lambda). The runner refuses to start if this fails.
- Secrets (`RUNNER_TOKEN`, `AWS_*` credentials) are deleted from the runner's
  environment as soon as they are read, so nothing it starts later (the
  toolchain probes) inherits them. This does **not** clean
  `/proc/<pid>/environ`, which shows the environment the process started
  with; that file is protected by non-dumpability alone. So in Lambda the
  role's credentials are assumed readable, and the role is built to be
  worthless: its own log group, its own ENIs, and an explicit deny on the ENI
  actions for calls made with the function's credentials
  (`lambda:SourceFunctionArn`). `AWS_LAMBDA_RUNTIME_API` is kept — it is an
  in-VM address, not a credential, and jobs cannot open TCP sockets to it.
- `PR_SET_CHILD_SUBREAPER = 1`: an orphaned double-forked grandchild is
  re-parented to the runner instead of init, so the reaper can always find it.

### 2. Per-job isolation (`engine.rs`)

- A fresh directory `…/job-<random>/` (random, never the caller's `job_id`, so
  no request can steer a path). Its layout and Unix permissions are documented
  at the top of `engine.rs`; the short version:
  - **root mode** (compose/ECS): the job runs as a **distinct unprivileged uid**
    from a pool (`20000 + slot`, supplementary groups cleared). The job dir is
    root-owned and only *readable* by the job; the job can write only its own
    build/scratch/case directories. After the build, outputs are **sealed**
    (chowned back to root, made read-only) so one case cannot alter the program
    another case runs. Only directories and single-link regular files the
    job's uid created are re-owned; anything else in the output (a symlink, a
    hard link to someone else's file) is removed, so sealing can never hand
    the job's group a file it could not read.
  - **Lambda / non-root**: the job shares the runner's uid, so ownership cannot
    separate them — instead jobs run **one at a time** and the teardown between
    them is the boundary.
- Between every case *and* at job end: the process group is killed, escapees
  are reaped (see below), the case directory and the shared temp dirs are
  swept. A `Drop` guard runs the whole teardown even on panic or a dropped
  request future.
- Each job's uid is tied to its execution slot, so "kill everything owned by
  this uid" is exactly "kill this job" — no PID bookkeeping.

### 3. Per-process confinement (`exec/linux.rs`)

Set in the child between `fork` and `exec` (async-signal-safe, no allocation):

- **`setsid`** — own process group, killable with one `kill(-pgid)`.
- **rlimits** — `RLIMIT_CPU` (timeout + 1s), `RLIMIT_FSIZE` (16 MiB),
  `RLIMIT_NOFILE` (256), `RLIMIT_STACK` (64 MiB, so deep recursion is a correct
  answer not a segfault), `RLIMIT_CORE` (0), `RLIMIT_NPROC` (job's own uid: a
  flat 256; shared uid: current + 256, so a fork bomb is capped without killing
  the runner), and `RLIMIT_AS` for the languages that tolerate it.
- inherited descriptors above stderr marked close-on-exec.
- **`PR_SET_NO_NEW_PRIVS`** — `exec` can never regain privileges; setuid bits
  and file capabilities are inert.
- **drop to the job uid/gid** (root mode), verified irreversible.
- **seccomp-BPF filter** (`exec/seccomp.rs`) — last. A denylist returning
  `EPERM` (so programs fail with a readable error, not `SIGSYS`): `socket` for
  any family except `AF_UNIX`; `ptrace`, `process_vm_readv/writev`,
  `process_madvise`, `pidfd_getfd`, `kcmp`; `mount`/`umount2`/`pivot_root`/
  `chroot`, the new mount API and `open_by_handle_at`; `unshare`/`setns`, and
  `clone` with any `CLONE_NEW*` flag; System V IPC (`shm*`, `sem*`, `msg*`)
  and POSIX message queues (`mq_*`), whose objects would outlive the job;
  `bpf`, `perf_event_open`, `userfaultfd`, `io_uring_*`,
  `keyctl`/`add_key`/`request_key`, `syslog`, `personality`, and the
  whole-machine operations (`kexec_*`, `*_module`, `reboot`, `swapon/off`).
  `clone3` answers `ENOSYS` instead: its flags are behind a pointer seccomp
  cannot read, and "not implemented" is what makes glibc and the runtimes fall
  back to the filtered `clone` (Docker's default profile does the same). On
  x86-64 the x32 ABI is refused wholesale so a `syscall | 0x40000000` cannot
  slip past a rule.

The job's program is not made non-dumpable: `exec` recomputes dumpability for
the new image, so it would not stick. Nothing relies on it — no other process
shares a job's uid (a private uid in root mode, one job at a time otherwise).

Then three watchdogs race the process (`exec/mod.rs`), and whichever fires
first kills the group and sets the verdict:

- **wall clock** per case and per compile (and a whole-job `total_timeout_ms`
  budget; cases that cannot start inside it are `skipped`);
- **output** — each stream capped at `max_output_bytes`, truncated on a UTF-8
  boundary, the first byte over the cap ends the run as `output_limit`;
- **resident memory** — the process group's RSS is polled every 20 ms (see
  [Memory](#memory) for why RSS and not only `RLIMIT_AS`).

### 4. Reaping and cache safety

- **Reaper** (`exec/linux.rs`): after each case and at teardown, repeated passes
  kill everything in scope until a pass finds nothing — by uid (root), by
  "not the runner or its ancestors" (Lambda), or by "re-parented to the runner"
  (dev). This is what stops the double-forked daemon in scenario 4.
- **Go build cache**: the standard library is compiled **once, at image-build
  time**, into a read-only, root-owned, world-readable cache (`/opt/dsa-runner/
  go-cache`, files `0444`). Each job gets a private `GOCACHE` built from
  hard links (same filesystem) or symlinks (across, e.g. Lambda's `/tmp`) to
  those files: it reads them and adds its own entries beside them, but cannot
  modify a shared inode — so it cannot plant a trojaned `fmt` for the next user
  (scenario 7). See `engine::link_farm`.

## Memory

A cgroup memory controller would be ideal but is unavailable to an unprivileged
container, so memory is capped two ways, chosen per language (`registry.rs`):

| language | mechanism | why |
| --- | --- | --- |
| C++, Python | `RLIMIT_AS` = budget | they allocate roughly what they use, and a failed `malloc` surfaces as `std::bad_alloc` / `MemoryError`, which the classifier turns into `memory_limit` |
| Go | `GOMEMLIMIT` (90% of budget) | Go reserves huge virtual address ranges; `RLIMIT_AS` would stop it starting |
| Java | `-Xmx` = budget − 96 MiB | the JVM maps far more than its heap; `-Xmx` caps the heap, 96 MiB covers metaspace/code-cache/stacks |

In **all** languages the RSS watchdog is the hard backstop: it measures what is
actually resident and kills the group a few tens of MiB over the limit. The
compile step gets its own, larger budget (`g++ -O2` on `<bits/stdc++.h>` needs
several hundred MiB) and is never squeezed into the program's budget.

## Known limitations

- **World-writable `/tmp`.** Without a mount namespace the sandbox cannot stop a
  program writing to `/tmp`, `/var/tmp`, `/dev/shm` or `/dev/mqueue`. It is
  mitigated, not eliminated: root mode sweeps everything the job's uid created
  there after each case and job; Lambda wipes those directories between jobs.
  Compose mounts `/tmp` as a size-capped `tmpfs` on a read-only root
  filesystem (so `/var/tmp` is not writable at all); Lambda already isolates
  `/tmp` per microVM. Concurrent jobs in one container still share the
  `tmpfs` space, so one job filling it can make another's writes fail until
  its case ends.
- **DNS / raw egress** rely on the deployment: the seccomp filter blocks
  `socket` for non-`AF_UNIX` families inside the process, but the definitive
  network barrier is the security group with no egress. Do not run the http
  transport on a host with an open egress path to anything sensitive.
- **Kernel bugs.** seccomp shrinks the attack surface but a kernel 0-day in an
  allowed syscall is out of scope; the microVM (Lambda) or host isolation is
  the backstop.
- **Signals to the runner (Lambda).** Sharing the runner's uid, a job can
  signal the runner — kill it, or stop it so its watchdogs stop too. That
  harms only the job's own invocation: Lambda's function timeout still ends
  it, and a new invocation gets a fresh execution environment if the runner
  died. Seccomp cannot tell the runner's pid from the job's own children.

## Running locally

```sh
# In the runner image (has the toolchains and the warm Go cache):
docker build -f backend/docker/runner.Dockerfile -t dsa-runner .
docker run --rm -p 8081:8081 dsa-runner
curl -s localhost:8081/healthz | jq

curl -s localhost:8081/v1/execute -H 'content-type: application/json' -d '{
  "protocol":1,"job_id":"demo","language":"python",
  "source":"print(sum(map(int,input().split())))",
  "cases":[{"id":"c1","stdin":"2 7 11 15"}]
}' | jq
```

On a non-Linux dev machine the crate builds and runs, but the sandbox is
Linux-only, so jobs are refused unless you opt into the **unsandboxed**
executor (no isolation — development only):

```sh
RUNNER_ALLOW_UNSANDBOXED=1 cargo run -p dsa-runner
```

### Tests

- Unit tests run everywhere: `cargo test -p dsa-runner`.
- The sandbox integration tests (`tests/sandbox_linux.rs`) need Linux, the
  toolchains, and root for the isolation assertions. Run them in the image:

  ```sh
  docker build -f backend/docker/runner.Dockerfile --target test -t dsa-runner-test .
  docker run --rm dsa-runner-test
  ```

  CI does exactly this (`platform-ci.yml`, job `runner-sandbox`), and also
  checks that a build naming no `--target` gets the runner: `runtime` is the
  Dockerfile's last stage on purpose, so compose and the CDK asset can never
  ship the test image.

## Adding a language

Languages are baked into the binary, never taken from a request. To add one:

1. Add a variant to `Language` in `registry.rs` and fill in `id`,
   `source_file`, `probe_commands`, `compile_argv`/`run_argv`, the environment,
   and the memory policy (`run_memory`/`compile_memory` — decide whether
   `RLIMIT_AS` suits the runtime or it needs a heap flag like Go/Java).
2. Add its toolchain to `backend/docker/runner.Dockerfile` (pinned and, for a
   tarball, checksum-verified), and — if it benefits from a warm cache — prime
   it at build time as Go does.
3. Add a hello-world and a two-sum case to `tests/sandbox_linux.rs`; the
   adversarial tests are language-agnostic and will pick it up where relevant.
4. Confirm the toolchain still works under the seccomp filter (run the tests);
   widen the denylist rather than the allowlist if it does not.

No protocol change is needed: the language id is just a new string the registry
recognises.
