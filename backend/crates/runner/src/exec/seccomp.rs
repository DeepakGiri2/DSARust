//! The seccomp-BPF filter every job process runs under.
//!
//! It is a **denylist** that answers `EPERM`: everything not listed is allowed,
//! and a listed syscall fails as if the kernel had refused it. Both choices are
//! deliberate.
//!
//! * An allowlist would be tighter, but four toolchains and their runtimes
//!   (Go's scheduler, the JVM, CPython, `cc1plus`) use a large and
//!   version-dependent set of syscalls; an allowlist that is wrong by one
//!   syscall breaks a language in a way no user can diagnose. What is denied
//!   here is what a DSA solution never needs and an escape usually does: the
//!   network, other processes' memory, namespaces and mounts, kernel objects
//!   that outlive their creator, and the kernel's most bug-prone interfaces.
//! * `EPERM` instead of killing: a program that tries to open a socket gets an
//!   ordinary error and a readable message ("operation not permitted"), not an
//!   unexplained SIGSYS.
//!
//! Three syscalls get more than a yes or no:
//!
//! * `socket` — `AF_UNIX` stays allowed (libc and runtimes use it locally, and
//!   it reaches nothing outside the machine); every other family — IPv4, IPv6,
//!   netlink, packet, vsock — is refused. That is the network barrier inside
//!   the process; the deployment adds one outside it (see the README).
//! * `clone` — refused when its flags ask for a new namespace. The flags are a
//!   plain register argument, so the filter can read them.
//! * `clone3` — always answers `ENOSYS`. Its flags sit in a struct behind a
//!   pointer, which seccomp cannot read, so it cannot be filtered by flag; and
//!   "not implemented" (unlike `EPERM`) is what glibc and the language
//!   runtimes take as "use `clone` instead", so threads and `fork` keep
//!   working through the filtered path. Docker's default profile does the
//!   same.
//!
//! On x86_64 the filter also refuses every **x32-ABI** syscall. x32 calls carry
//! `AUDIT_ARCH_X86_64` like native ones but are numbered with bit 30 set, so a
//! denylist keyed on native numbers would not match them: without this check,
//! `socket | 0x4000_0000` would walk straight past the `socket` rule on a
//! kernel built with x32 support. (i386 calls via `int 0x80` carry a different
//! arch and are killed by seccompiler's own architecture check.)

use anyhow::{Context, Result};
use seccompiler::{
    sock_filter, BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition,
    SeccompFilter, SeccompRule, TargetArch,
};
use std::collections::BTreeMap;

#[cfg(target_arch = "x86_64")]
const ARCH: TargetArch = TargetArch::x86_64;
#[cfg(target_arch = "aarch64")]
const ARCH: TargetArch = TargetArch::aarch64;

/// Syscalls refused unconditionally, grouped by what they would give a job.
pub const DENIED: &[(&str, i64)] = &[
    // Reading or steering other processes (the runner included).
    ("ptrace", libc::SYS_ptrace),
    ("process_vm_readv", libc::SYS_process_vm_readv),
    ("process_vm_writev", libc::SYS_process_vm_writev),
    ("process_madvise", libc::SYS_process_madvise),
    ("pidfd_getfd", libc::SYS_pidfd_getfd),
    ("kcmp", libc::SYS_kcmp),
    // Rearranging the filesystem view, old and new mount APIs, and opening a
    // file by handle, which skips the permission checks of its path.
    ("mount", libc::SYS_mount),
    ("umount2", libc::SYS_umount2),
    ("pivot_root", libc::SYS_pivot_root),
    ("chroot", libc::SYS_chroot),
    ("open_tree", libc::SYS_open_tree),
    ("move_mount", libc::SYS_move_mount),
    ("fsopen", libc::SYS_fsopen),
    ("fsconfig", libc::SYS_fsconfig),
    ("fsmount", libc::SYS_fsmount),
    ("fspick", libc::SYS_fspick),
    ("mount_setattr", libc::SYS_mount_setattr),
    ("open_by_handle_at", libc::SYS_open_by_handle_at),
    // Namespaces: the usual first step of a container escape. (`clone` with a
    // namespace flag is refused by its own rule, `clone3` by the prelude.)
    ("unshare", libc::SYS_unshare),
    ("setns", libc::SYS_setns),
    // Kernel objects that outlive the process that made them. A System V
    // shared-memory segment, semaphore set or message queue, and a POSIX
    // message queue, stay until something removes them: memory a job would
    // keep after its teardown, and a place to leave data for a later job.
    ("shmget", libc::SYS_shmget),
    ("shmat", libc::SYS_shmat),
    ("shmctl", libc::SYS_shmctl),
    ("semget", libc::SYS_semget),
    ("semop", libc::SYS_semop),
    ("semtimedop", libc::SYS_semtimedop),
    ("semctl", libc::SYS_semctl),
    ("msgget", libc::SYS_msgget),
    ("msgsnd", libc::SYS_msgsnd),
    ("msgrcv", libc::SYS_msgrcv),
    ("msgctl", libc::SYS_msgctl),
    ("mq_open", libc::SYS_mq_open),
    ("mq_unlink", libc::SYS_mq_unlink),
    ("mq_timedsend", libc::SYS_mq_timedsend),
    ("mq_timedreceive", libc::SYS_mq_timedreceive),
    ("mq_notify", libc::SYS_mq_notify),
    ("mq_getsetattr", libc::SYS_mq_getsetattr),
    // Large, historically bug-prone kernel interfaces no solution needs.
    ("bpf", libc::SYS_bpf),
    ("perf_event_open", libc::SYS_perf_event_open),
    ("userfaultfd", libc::SYS_userfaultfd),
    ("io_uring_setup", libc::SYS_io_uring_setup),
    ("io_uring_enter", libc::SYS_io_uring_enter),
    ("io_uring_register", libc::SYS_io_uring_register),
    ("keyctl", libc::SYS_keyctl),
    ("add_key", libc::SYS_add_key),
    ("request_key", libc::SYS_request_key),
    // The kernel log: addresses, and messages about everything else running.
    ("syslog", libc::SYS_syslog),
    // Execution-domain changes (e.g. turning off ASLR for the next exec).
    ("personality", libc::SYS_personality),
    // Whole-machine operations. Privileged anyway; refused so a
    // misconfigured deployment still cannot reach them.
    ("kexec_load", libc::SYS_kexec_load),
    ("kexec_file_load", libc::SYS_kexec_file_load),
    ("init_module", libc::SYS_init_module),
    ("finit_module", libc::SYS_finit_module),
    ("delete_module", libc::SYS_delete_module),
    ("reboot", libc::SYS_reboot),
    ("swapon", libc::SYS_swapon),
    ("swapoff", libc::SYS_swapoff),
];

/// The namespace flags `clone` may not carry. `CLONE_NEWTIME` is not here on
/// purpose: `clone` has no room for it (its bit is part of the exit-signal
/// byte there); only `clone3` and `unshare` accept it, and both are refused.
pub const CLONE_NAMESPACE_FLAGS: [libc::c_int; 7] = [
    libc::CLONE_NEWNS,
    libc::CLONE_NEWCGROUP,
    libc::CLONE_NEWUTS,
    libc::CLONE_NEWIPC,
    libc::CLONE_NEWUSER,
    libc::CLONE_NEWPID,
    libc::CLONE_NEWNET,
];

/// Compile the filter. Done once at startup; a failure there is fatal,
/// because the runner never executes jobs without it on Linux.
pub fn build() -> Result<BpfProgram> {
    build_with(libc::EPERM, libc::ENOSYS)
}

/// [`build`] with its two answers as parameters: `refuse` for everything
/// denied, `unsupported` for `clone3`. Tests pass errnos nothing else uses,
/// so they can tell this filter's refusal from that of another filter stacked
/// with it (Docker's default profile refuses some of the same calls, with the
/// same errnos).
fn build_with(refuse: i32, unsupported: i32) -> Result<BpfProgram> {
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> =
        DENIED.iter().map(|&(_, nr)| (nr, Vec::new())).collect();

    // socket(domain, …) with domain != AF_UNIX. `domain` is an int, so a
    // 32-bit comparison of argument 0.
    let not_unix = SeccompCondition::new(
        0,
        SeccompCmpArgLen::Dword,
        SeccompCmpOp::Ne,
        libc::AF_UNIX as u64,
    )
    .context("seccomp: socket condition")?;
    rules.insert(
        libc::SYS_socket,
        vec![SeccompRule::new(vec![not_unix]).context("seccomp: socket rule")?],
    );

    // clone(flags, …) with any namespace flag set: one rule per flag, and a
    // syscall matches when any one of its rules does. Every namespace flag
    // lives in the low 32 bits of `flags` (argument 0 on both architectures).
    let new_namespace = CLONE_NAMESPACE_FLAGS
        .iter()
        .map(|&flag| {
            let flag = flag as u64;
            let set = SeccompCondition::new(
                0,
                SeccompCmpArgLen::Dword,
                SeccompCmpOp::MaskedEq(flag),
                flag,
            )?;
            SeccompRule::new(vec![set])
        })
        .collect::<Result<Vec<_>, _>>()
        .context("seccomp: clone rules")?;
    rules.insert(libc::SYS_clone, new_namespace);

    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(refuse as u32),
        ARCH,
    )
    .context("seccomp: filter")?;
    let program: BpfProgram = filter.try_into().context("seccomp: compile")?;
    Ok(with_prelude(program, refuse as u32, unsupported as u32))
}

const LD_W_ABS: u16 = 0x20; // BPF_LD | BPF_W | BPF_ABS
const JEQ_K: u16 = 0x15; // BPF_JMP | BPF_JEQ | BPF_K
const RET_K: u16 = 0x06; // BPF_RET | BPF_K
const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;

fn insn(code: u16, jt: u8, jf: u8, k: u32) -> sock_filter {
    sock_filter { code, jt, jf, k }
}

/// Hand-written checks that run before seccompiler's program, on the syscall
/// number alone (loaded once into the accumulator). seccompiler's jumps are
/// relative, so its program is unaffected by what runs before it; it reloads
/// what it needs, starting with the arch check.
fn with_prelude(program: BpfProgram, refuse: u32, unsupported: u32) -> BpfProgram {
    let mut prelude = vec![
        // A = seccomp_data.nr (offset 0)
        insn(LD_W_ABS, 0, 0, 0),
        // clone3 → `unsupported`. 435 is clone3 on every ABI that can reach
        // this filter (x86_64, arm64, and their 32-bit compat ABIs), so the
        // arch need not be checked first; any other arch is killed below.
        insn(JEQ_K, 0, 1, libc::SYS_clone3 as u32),
        insn(RET_K, 0, 0, SECCOMP_RET_ERRNO | unsupported),
    ];
    prelude.extend(x32_guard(refuse));
    prelude.into_iter().chain(program).collect()
}

#[cfg(target_arch = "x86_64")]
fn x32_guard(refuse: u32) -> Vec<sock_filter> {
    const JGE_K: u16 = 0x35; // BPF_JMP | BPF_JGE | BPF_K
    const X32_SYSCALL_BIT: u32 = 0x4000_0000;
    vec![
        // A (still the syscall number) >= X32_SYSCALL_BIT: refuse; else skip.
        insn(JGE_K, 0, 1, X32_SYSCALL_BIT),
        insn(RET_K, 0, 0, SECCOMP_RET_ERRNO | refuse),
    ]
}

#[cfg(not(target_arch = "x86_64"))]
fn x32_guard(_refuse: u32) -> Vec<sock_filter> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use libc::c_long;

    /// Errnos no kernel path or other filter returns for these calls. The
    /// kernel keeps the most recently installed filter's errno when stacked
    /// filters return the same action, so under Docker's default profile a
    /// check that sees these knows *this* filter answered.
    const TEST_REFUSE: i32 = libc::EL2HLT;
    const TEST_UNSUPPORTED: i32 = libc::EL3HLT;

    #[test]
    fn the_filter_compiles_and_names_each_syscall_once() {
        let program = build().expect("filter builds");
        assert!(!program.is_empty());
        assert!(
            program.len() < 4096,
            "BPF programs are limited to 4096 instructions"
        );
        let mut numbers: Vec<i64> = DENIED.iter().map(|&(_, nr)| nr).collect();
        numbers.sort_unstable();
        numbers.dedup();
        assert_eq!(numbers.len(), DENIED.len());
        // Conditional (socket, clone), answered separately (clone3), or
        // needed by every program (execve, socketpair): none may be denied
        // outright.
        for kept in [
            libc::SYS_socket,
            libc::SYS_clone,
            libc::SYS_clone3,
            libc::SYS_execve,
            libc::SYS_socketpair,
        ] {
            assert!(
                !numbers.contains(&kept),
                "{kept} must not be denied outright"
            );
        }
    }

    /// Fork, install `program` in the child behind `no_new_privs` (as the
    /// executor does), and run `checks` there. Returns 0 when every check
    /// passed, else the number of the first that failed. The child makes only
    /// raw syscalls on data prepared before the fork and leaves with `_exit`,
    /// never returning into the test harness.
    fn in_filtered_child(program: &BpfProgram, checks: unsafe fn() -> i32) -> i32 {
        // SAFETY: as described above; `checks` obeys the same rules.
        unsafe {
            let pid = libc::fork();
            assert!(pid >= 0, "fork failed");
            if pid == 0 {
                if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
                    || seccompiler::apply_filter(program).is_err()
                {
                    libc::_exit(100);
                }
                libc::_exit(checks());
            }
            let mut status = 0;
            assert_eq!(libc::waitpid(pid, &mut status, 0), pid);
            assert!(libc::WIFEXITED(status), "child died: status {status:#x}");
            libc::WEXITSTATUS(status)
        }
    }

    unsafe fn errno() -> i32 {
        *libc::__errno_location()
    }

    unsafe fn refused(result: c_long) -> bool {
        result == -1 && errno() == TEST_REFUSE
    }

    /// `clone(flags | SIGCHLD)` through the raw syscall, fork-style (no new
    /// stack). A child it creates leaves at once, and is waited for, so a
    /// call the filter wrongly let through cannot run the checks twice.
    unsafe fn raw_clone(flags: libc::c_int) -> c_long {
        let zero: c_long = 0;
        let r = libc::syscall(
            libc::SYS_clone,
            c_long::from(flags | libc::SIGCHLD),
            zero,
            zero,
            zero,
            zero,
        );
        if r == 0 {
            libc::_exit(0);
        }
        if r > 0 {
            libc::waitpid(r as libc::pid_t, std::ptr::null_mut(), 0);
        }
        r
    }

    unsafe fn refusal_checks() -> i32 {
        // The network: every family but AF_UNIX.
        if !refused(libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0).into()) {
            return 1;
        }
        if !refused(libc::socket(libc::AF_INET6, libc::SOCK_DGRAM, 0).into()) {
            return 2;
        }
        if !refused(libc::socket(libc::AF_NETLINK, libc::SOCK_RAW, 0).into()) {
            return 3;
        }
        // Local IPC is not caught by the socket rule.
        if libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) < 0 {
            return 4;
        }
        let mut pair = [0; 2];
        if libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, pair.as_mut_ptr()) != 0 {
            return 5;
        }
        // Other processes, namespaces, a bug-prone interface.
        if !refused(libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0)) {
            return 6;
        }
        if !refused(libc::unshare(libc::CLONE_NEWUSER).into()) {
            return 7;
        }
        if !refused(libc::syscall(
            libc::SYS_io_uring_setup,
            1 as c_long,
            0 as c_long,
        )) {
            return 8;
        }
        #[cfg(target_arch = "x86_64")]
        if !refused(libc::syscall(
            libc::SYS_socket | 0x4000_0000,
            c_long::from(libc::AF_INET),
            1 as c_long,
            0 as c_long,
        )) {
            return 9;
        }
        // clone3 gets its own answer. A null struct would be EINVAL/EFAULT
        // without the filter, so no child can result from a failed check.
        let r = libc::syscall(libc::SYS_clone3, 0 as c_long, 0 as c_long);
        if !(r == -1 && errno() == TEST_UNSUPPORTED) {
            return 10;
        }
        // clone with each namespace flag on its own is refused ...
        for (i, &flag) in CLONE_NAMESPACE_FLAGS.iter().enumerate() {
            if !refused(raw_clone(flag)) {
                return 11 + i as i32; // 11..=17
            }
        }
        // ... while clone without one, and fork, still create children.
        if raw_clone(0) <= 0 {
            return 18;
        }
        let pid = libc::fork();
        if pid == 0 {
            libc::_exit(0);
        }
        if pid < 0 || libc::waitpid(pid, std::ptr::null_mut(), 0) != pid {
            return 19;
        }
        // Kernel objects that would outlive the job.
        let private = libc::IPC_PRIVATE;
        let create = libc::IPC_CREAT | 0o600;
        if !refused(libc::shmget(private, 4096, create).into()) {
            return 20;
        }
        if !refused(libc::semget(private, 1, create).into()) {
            return 21;
        }
        if !refused(libc::msgget(private, create).into()) {
            return 22;
        }
        let queue = b"dsa-runner-seccomp-test\0";
        if !refused(libc::syscall(
            libc::SYS_mq_open,
            queue.as_ptr() as c_long,
            c_long::from(libc::O_CREAT | libc::O_RDWR),
            0o600 as c_long,
            0 as c_long,
        )) {
            return 23;
        }
        // Everything else is untouched.
        if libc::getpid() <= 0 {
            return 24;
        }
        0
    }

    /// Every rule, checked with errnos only this filter returns.
    #[test]
    fn the_filter_refuses_the_network_escapes_and_lasting_ipc_but_not_local_ipc_or_fork() {
        let program = build_with(TEST_REFUSE, TEST_UNSUPPORTED).unwrap();
        let failed = in_filtered_child(&program, refusal_checks);
        assert_eq!(
            failed, 0,
            "check {failed} failed (numbered in refusal_checks)"
        );
    }

    unsafe fn real_answer_checks() -> i32 {
        if !(libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) == -1 && errno() == libc::EPERM) {
            return 1;
        }
        let r = libc::syscall(libc::SYS_clone3, 0 as c_long, 0 as c_long);
        if !(r == -1 && errno() == libc::ENOSYS) {
            return 2;
        }
        // posix_spawn creates its child through glibc's clone3-first helper,
        // the same one pthread_create uses; it only falls back to clone on
        // ENOSYS, so this fails if clone3 is refused any other way.
        let sh = b"/bin/sh\0".as_ptr().cast::<libc::c_char>();
        let argv: [*mut libc::c_char; 4] = [
            b"sh\0".as_ptr().cast_mut().cast(),
            b"-c\0".as_ptr().cast_mut().cast(),
            b"exit 0\0".as_ptr().cast_mut().cast(),
            std::ptr::null_mut(),
        ];
        let envp: [*mut libc::c_char; 1] = [std::ptr::null_mut()];
        let mut pid: libc::pid_t = 0;
        if libc::posix_spawn(
            &mut pid,
            sh,
            std::ptr::null(),
            std::ptr::null(),
            argv.as_ptr(),
            envp.as_ptr(),
        ) != 0
        {
            return 3;
        }
        let mut status = 0;
        if libc::waitpid(pid, &mut status, 0) != pid
            || !libc::WIFEXITED(status)
            || libc::WEXITSTATUS(status) != 0
        {
            return 4;
        }
        0
    }

    /// The production answers, and that ENOSYS for clone3 keeps process and
    /// thread creation working.
    #[test]
    fn the_real_filter_answers_eperm_and_lets_glibc_fall_back_from_clone3() {
        let failed = in_filtered_child(&build().unwrap(), real_answer_checks);
        assert_eq!(
            failed, 0,
            "check {failed} failed (numbered in real_answer_checks)"
        );
    }
}
