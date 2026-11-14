// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! The C ABI the Qt application links against.
//!
//! Design rules, all of them load-bearing:
//!
//! - **No panic crosses the boundary.** Every entry point wraps its body in
//!   `catch_unwind`; a panic becomes an error return, so the UI and the mount
//!   stay alive (invariant 7).
//! - **Strings are owned by the caller.** Output strings are allocated here and
//!   released with [`nidvue_string_free`].
//! - **State crosses as JSON.** The UI never interprets a Rust type; it parses
//!   the same schema `nidvue-sdk` publishes.
//! - **The UI thread never blocks on I/O.** Long calls (`login`, `sync_now`)
//!   block the calling thread only; the Qt bridge calls them from a worker.
//!
//! The header is `include/nidvue.h`; `tests/header.rs` checks that every
//! declared symbol exists here.

use std::ffi::{c_char, c_void, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::{Arc, Mutex};

use nidvue_mount::MountHandle;
use nidvue_sdk::{Client, ConflictPolicy};
use tokio::runtime::Runtime;

/// Success.
pub const NIDVUE_OK: i32 = 0;
/// A generic failure; read the error string.
pub const NIDVUE_ERR: i32 = -1;
/// A panic was contained.
pub const NIDVUE_PANIC: i32 = -2;
/// A null pointer or invalid UTF-8 argument.
pub const NIDVUE_INVALID: i32 = -3;

type EventCallback = extern "C" fn(json: *const c_char, userdata: *mut c_void);

struct Subscription {
    callback: EventCallback,
    userdata: usize,
}

/// The opaque client handle the C side holds.
pub struct NidvueClient {
    runtime: Runtime,
    client: Client,
    mount: Mutex<Option<MountHandle>>,
    subscription: Arc<Mutex<Option<Subscription>>>,
}

fn set_err(err: *mut *mut c_char, message: impl Into<String>) {
    if err.is_null() {
        return;
    }
    let message = message.into();
    let text = CString::new(message.replace('\0', " ")).unwrap_or_default();
    // SAFETY: the caller passed a valid out-pointer; we overwrite it once.
    unsafe { *err = text.into_raw() };
}

fn cstr<'a>(value: *const c_char) -> Option<&'a str> {
    if value.is_null() {
        return None;
    }
    // SAFETY: the caller guarantees a NUL-terminated string for the call.
    unsafe { CStr::from_ptr(value) }.to_str().ok()
}

/// Free a string returned by this library. Null is a no-op.
///
/// # Safety
/// `value` must come from this library and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn nidvue_string_free(value: *mut c_char) {
    if !value.is_null() {
        drop(CString::from_raw(value));
    }
}

/// Open a client over a state directory.
///
/// # Safety
/// `state_dir` must be a valid NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_new(
    state_dir: *const c_char,
    err: *mut *mut c_char,
) -> *mut NidvueClient {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let Some(dir) = cstr(state_dir) else {
            set_err(err, "state_dir is null or not UTF-8");
            return ptr::null_mut();
        };
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                set_err(err, format!("runtime: {e}"));
                return ptr::null_mut();
            }
        };
        match runtime.block_on(Client::open(dir)) {
            Ok(client) => Box::into_raw(Box::new(NidvueClient {
                runtime,
                client,
                mount: Mutex::new(None),
                subscription: Arc::new(Mutex::new(None)),
            })),
            Err(e) => {
                set_err(err, e.to_string());
                ptr::null_mut()
            }
        }
    }));
    result.unwrap_or_else(|_| {
        set_err(err, "panic contained in nidvue_client_new");
        ptr::null_mut()
    })
}

/// Close a client and unmount if needed.
///
/// # Safety
/// `client` must come from [`nidvue_client_new`] and not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_free(client: *mut NidvueClient) {
    if client.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let boxed = Box::from_raw(client);
        if let Ok(mut mount) = boxed.mount.lock() {
            if let Some(handle) = mount.take() {
                drop(handle);
            }
        }
        drop(boxed);
    }));
}

/// Exchange a Personal Login Token.
///
/// # Safety
/// `client` and `token` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_login(
    client: *mut NidvueClient,
    token: *const c_char,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    let Some(token) = cstr(token) else {
        set_err(err, "token is null or not UTF-8");
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client.runtime.block_on(client.client.login(token))
    })) {
        Ok(Ok(_username)) => NIDVUE_OK,
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_login");
            NIDVUE_PANIC
        }
    }
}

/// Forget the stored token.
///
/// # Safety
/// `client` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_logout(
    client: *mut NidvueClient,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client.runtime.block_on(client.client.logout())
    })) {
        Ok(Ok(())) => NIDVUE_OK,
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_logout");
            NIDVUE_PANIC
        }
    }
}

fn json_out(out: *mut *mut c_char, value: String) -> i32 {
    if out.is_null() {
        return NIDVUE_INVALID;
    }
    match CString::new(value) {
        Ok(text) => {
            // SAFETY: the caller passed a valid out-pointer.
            unsafe { *out = text.into_raw() };
            NIDVUE_OK
        }
        Err(_) => NIDVUE_INVALID,
    }
}

/// `status --json`, allocated for the caller.
///
/// # Safety
/// `client` must be valid; `out` must be a valid out-pointer.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_status_json(
    client: *mut NidvueClient,
    out: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client
            .runtime
            .block_on(client.client.status())
            .and_then(|status| {
                serde_json::to_string(&status)
                    .map_err(|e| nidvue_core::NidvueError::Internal(format!("status json: {e}")))
            })
    })) {
        Ok(Ok(json)) => json_out(out, json),
        Ok(Err(_)) | Err(_) => NIDVUE_ERR,
    }
}

/// Mount at `mountpoint`.
///
/// # Safety
/// `client` and `mountpoint` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_mount(
    client: *mut NidvueClient,
    mountpoint: *const c_char,
    read_only: i32,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    let Some(mountpoint) = cstr(mountpoint) else {
        set_err(err, "mountpoint is null or not UTF-8");
        return NIDVUE_INVALID;
    };
    let options = nidvue_mount::MountOptions {
        read_only: read_only != 0,
        ..Default::default()
    };
    match catch_unwind(AssertUnwindSafe(|| {
        let _guard = client.runtime.enter();
        client
            .client
            .mount(std::path::Path::new(mountpoint), options)
    })) {
        Ok(Ok(handle)) => match client.mount.lock() {
            Ok(mut slot) => {
                *slot = Some(handle);
                NIDVUE_OK
            }
            Err(_) => NIDVUE_ERR,
        },
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_mount");
            NIDVUE_PANIC
        }
    }
}

/// Unmount, if mounted.
///
/// # Safety
/// `client` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_unmount(client: *mut NidvueClient) -> i32 {
    let Some(client) = client.as_ref() else {
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        if let Ok(mut slot) = client.mount.lock() {
            slot.take();
        }
    })) {
        Ok(()) => NIDVUE_OK,
        Err(_) => NIDVUE_PANIC,
    }
}

/// Run one sync pass over every configured pair; returns a JSON array of
/// reports.
///
/// # Safety
/// `client` must be valid; `out` must be a valid out-pointer.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_sync_now(
    client: *mut NidvueClient,
    out: *mut *mut c_char,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client
            .runtime
            .block_on(client.client.sync_now())
            .and_then(|reports| {
                serde_json::to_string(
                    &reports
                        .iter()
                        .map(|r| {
                            serde_json::json!({
                                "scanned": r.scanned,
                                "planned": r.planned,
                                "applied": r.applied,
                                "conflicts": r.conflicts,
                                "failed": r.failed.len(),
                            })
                        })
                        .collect::<Vec<_>>(),
                )
                .map_err(|e| nidvue_core::NidvueError::Internal(format!("sync json: {e}")))
            })
    })) {
        Ok(Ok(json)) => json_out(out, json),
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_sync_now");
            NIDVUE_PANIC
        }
    }
}

/// Open conflicts as a JSON array.
///
/// # Safety
/// `client` must be valid; `out` must be a valid out-pointer.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_conflicts_json(
    client: *mut NidvueClient,
    out: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client.client.conflicts().and_then(|rows| {
            serde_json::to_string(&rows)
                .map_err(|e| nidvue_core::NidvueError::Internal(format!("conflicts json: {e}")))
        })
    })) {
        Ok(Ok(json)) => json_out(out, json),
        Ok(Err(_)) | Err(_) => NIDVUE_ERR,
    }
}

/// Resolve one conflict: `keep_local`, `keep_remote` or `keep_both`.
///
/// # Safety
/// `client`, `id` and `resolution` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_resolve_conflict(
    client: *mut NidvueClient,
    id: *const c_char,
    resolution: *const c_char,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    let (Some(id), Some(resolution)) = (cstr(id), cstr(resolution)) else {
        set_err(err, "id or resolution is null or not UTF-8");
        return NIDVUE_INVALID;
    };
    let policy = match ConflictPolicy::parse(resolution) {
        Some(p) => p,
        None => {
            set_err(err, format!("unknown resolution {resolution:?}"));
            return NIDVUE_INVALID;
        }
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client.client.resolve_conflict(id, policy)
    })) {
        Ok(Ok(())) => NIDVUE_OK,
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_resolve_conflict");
            NIDVUE_PANIC
        }
    }
}

/// Subscribe to typed events. The callback runs on a Rust thread; the Qt side
/// must marshal it onto the UI thread.
///
/// # Safety
/// `callback` must be safe to call from another thread; `userdata` must stay
/// valid until the client is freed.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_subscribe(
    client: *mut NidvueClient,
    callback: Option<EventCallback>,
    userdata: *mut c_void,
) -> i32 {
    let Some(client) = client.as_ref() else {
        return NIDVUE_INVALID;
    };
    let Some(callback) = callback else {
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        if let Ok(mut slot) = client.subscription.lock() {
            *slot = Some(Subscription {
                callback,
                userdata: userdata as usize,
            });
        }
        let mut rx = client.client.subscribe();
        let slot = client.subscription.clone();
        client.runtime.spawn(async move {
            while let Ok(envelope) = rx.recv().await {
                let Ok(json) = serde_json::to_string(&envelope) else {
                    continue;
                };
                let Ok(text) = CString::new(json) else {
                    continue;
                };
                let target = slot
                    .lock()
                    .ok()
                    .and_then(|s| s.as_ref().map(|s| (s.callback, s.userdata)));
                if let Some((callback, userdata)) = target {
                    callback(text.as_ptr(), userdata as *mut c_void);
                }
            }
        });
    })) {
        Ok(()) => NIDVUE_OK,
        Err(_) => NIDVUE_PANIC,
    }
}

/// The JSON schema for events and configuration, allocated for the caller.
///
/// # Safety
/// `out` must be a valid out-pointer.
#[no_mangle]
pub unsafe extern "C" fn nidvue_schema_json(out: *mut *mut c_char) -> i32 {
    match catch_unwind(AssertUnwindSafe(Client::json_schema)) {
        Ok(json) => json_out(out, json),
        Err(_) => NIDVUE_PANIC,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(value: *mut c_char) -> String {
        // SAFETY: the pointer came from this library and is freed once.
        unsafe {
            let text = CStr::from_ptr(value).to_string_lossy().into_owned();
            nidvue_string_free(value);
            text
        }
    }

    #[test]
    fn opens_reports_and_frees() {
        let dir = tempfile::tempdir().unwrap();
        let state = CString::new(dir.path().to_str().unwrap()).unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        // SAFETY: all pointers are valid for the duration of the test.
        unsafe {
            let client = nidvue_client_new(state.as_ptr(), &mut err);
            assert!(!client.is_null(), "client is null");
            assert!(err.is_null());
            let mut out: *mut c_char = ptr::null_mut();
            assert_eq!(nidvue_client_status_json(client, &mut out), NIDVUE_OK);
            let status = take(out);
            assert!(status.contains("\"logged_in\":false"), "{status}");
            let mut conflicts: *mut c_char = ptr::null_mut();
            assert_eq!(
                nidvue_client_conflicts_json(client, &mut conflicts),
                NIDVUE_OK
            );
            assert_eq!(take(conflicts), "[]");
            nidvue_client_free(client);
        }
    }

    #[test]
    fn null_arguments_are_refused_not_dereferenced() {
        let mut err: *mut c_char = ptr::null_mut();
        // SAFETY: null is explicitly handled.
        unsafe {
            assert!(nidvue_client_new(ptr::null(), &mut err).is_null());
            assert_eq!(
                nidvue_client_login(ptr::null_mut(), ptr::null(), &mut err),
                NIDVUE_INVALID
            );
            assert_eq!(nidvue_client_unmount(ptr::null_mut()), NIDVUE_INVALID);
            nidvue_client_free(ptr::null_mut());
            nidvue_string_free(ptr::null_mut());
            if !err.is_null() {
                take(err);
            }
        }
    }

    #[test]
    fn schema_is_available_without_a_client() {
        let mut out: *mut c_char = ptr::null_mut();
        // SAFETY: `out` is a valid out-pointer.
        unsafe {
            assert_eq!(nidvue_schema_json(&mut out), NIDVUE_OK);
            let schema = take(out);
            assert!(schema.contains("\"event\""));
        }
    }

    #[test]
    fn a_bad_state_directory_reports_an_error() {
        // A path below a regular file cannot be created on any platform.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-a-dir");
        std::fs::write(&file, b"x").unwrap();
        let bad = file.join("state");
        let path = CString::new(bad.to_str().unwrap()).unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        // SAFETY: valid pointers for the call.
        unsafe {
            let client = nidvue_client_new(path.as_ptr(), &mut err);
            assert!(client.is_null());
            assert!(!err.is_null());
            let message = take(err);
            assert!(!message.is_empty());
        }
    }
}
