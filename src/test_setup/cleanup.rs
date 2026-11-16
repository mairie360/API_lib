//! Removal of the test containers when the test binary exits.
//!
//! A container held by a `static` (the shared Postgres and Redis) is never dropped, so testcontainers
//! would not remove it and it would outlive the binary: one `atexit` hook removes all of them.

use std::process::{Command, Stdio};
use std::sync::{Mutex, Once};

/// Ids of the containers to remove at exit.
static CONTAINER_IDS: Mutex<Vec<String>> = Mutex::new(Vec::new());

static REGISTER_HOOK: Once = Once::new();

extern "C" fn remove_containers() {
    // `try_lock`: never block (nor panic) in an exit hook, a poisoned or busy list is left as is.
    let Ok(ids) = CONTAINER_IDS.try_lock() else {
        return;
    };
    for id in ids.iter() {
        let _ = Command::new("docker")
            .args(["rm", "--force", "--volumes", id])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// Removes the container `id` (and its volumes) when the process exits.
pub(super) fn remove_at_exit(id: &str) {
    if let Ok(mut ids) = CONTAINER_IDS.lock() {
        ids.push(id.to_owned());
    }
    REGISTER_HOOK.call_once(|| {
        // SAFETY: `remove_containers` is an `extern "C"` function without argument that does not
        // panic; `atexit` only registers it.
        // nosemgrep: rust.lang.security.unsafe-usage.unsafe-usage -- test-only helper, see SAFETY above
        if unsafe { libc::atexit(remove_containers) } != 0 {
            eprintln!("⚠️ Could not register the removal of the test containers at exit");
        }
    });
}
