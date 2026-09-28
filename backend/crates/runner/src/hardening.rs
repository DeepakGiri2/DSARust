//! Hardening the runner process itself, once, at startup.
//!
//! Jobs never inherit the runner's environment (every child starts from
//! `env_clear`), but in Lambda they run as the runner's own uid, and a same-uid
//! process can normally read `/proc/<runner>/environ` and attach a debugger.
//! Making the runner non-dumpable closes that, and `main` refuses to start the
//! runner when it cannot.
//!
//! Deleting the secrets from the environment once the configuration has been
//! read is a second, narrower measure: nothing the runner starts afterwards
//! (the toolchain probes do not clear their environment) and nothing that
//! looks a variable up later can see them. It does *not* clean
//! `/proc/<runner>/environ`, which shows the block the process was started
//! with, not its current environment; that file is protected by
//! non-dumpability alone. In Lambda the role's credentials are therefore
//! assumed readable by a job regardless, and the role is built so they are
//! worth nothing (see the runner stack in `infra/`).

/// Variables that must not survive in the runner's own environment once the
/// configuration has been read. `AWS_LAMBDA_RUNTIME_API` is deliberately not
/// here: the Lambda client needs it, and it is an address, not a credential
/// (it is reachable only from inside the execution environment).
pub const SECRET_VARS: &[&str] = &[
    "RUNNER_TOKEN",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "AWS_SECURITY_TOKEN",
    "AWS_CONTAINER_AUTHORIZATION_TOKEN",
    "AWS_CONTAINER_CREDENTIALS_FULL_URI",
    "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
];

/// Remove [`SECRET_VARS`] from the process environment.
///
/// Call after `Config::from_env` and before any thread other than the main
/// one exists: mutating the environment is only sound while nothing else can
/// be reading it concurrently.
pub fn scrub_secrets() {
    for key in SECRET_VARS {
        std::env::remove_var(key);
    }
}

/// Make the runner non-dumpable and a child subreaper (Linux; a no-op
/// elsewhere).
///
/// * Non-dumpable: `/proc/<runner>/{environ,mem,maps,…}` become root-only and
///   `ptrace` attach is refused, even for processes of the same uid.
/// * Subreaper: an orphaned descendant — a job's double-forked daemon — is
///   re-parented to the runner instead of init, so the reaper can find it
///   (and reap its zombie) in every mode, not only when jobs have their own
///   uid.
pub fn harden_process() -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: prctl with integer arguments has no memory-safety
        // preconditions.
        unsafe {
            if libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) != 0
                || libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
        }
    }
    Ok(())
}

/// Whether the runner can give each job its own uid.
pub fn is_root() -> bool {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: geteuid has no preconditions.
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_credential_the_platform_injects_is_scrubbed() {
        for key in ["RUNNER_TOKEN", "AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN"] {
            assert!(SECRET_VARS.contains(&key), "{key}");
        }
        assert!(
            !SECRET_VARS.contains(&"AWS_LAMBDA_RUNTIME_API"),
            "the Lambda client still needs its endpoint"
        );
    }
}
