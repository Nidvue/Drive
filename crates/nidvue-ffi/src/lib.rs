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

use nidvue_core::config::SyncPair;
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

fn report_json(report: &nidvue_sdk::SyncReport) -> serde_json::Value {
    serde_json::json!({
        "pair": report.pair,
        "scanned": report.scanned,
        "planned": report.planned,
        "applied": report.applied,
        "conflicts": report.conflicts,
        "failed": report.failed.len(),
        "errors": report.failed.iter().map(|(path, why)| serde_json::json!({
            "path": path,
            "error": why,
        })).collect::<Vec<_>>(),
    })
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

/// Set the cache cap in bytes.
///
/// # Safety
/// `client` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_set_cache_limit(
    client: *mut NidvueClient,
    bytes: u64,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client.client.set_cache_limit_bytes(bytes)
    })) {
        Ok(Ok(())) => NIDVUE_OK,
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_set_cache_limit");
            NIDVUE_PANIC
        }
    }
}

/// Installed plugins as a JSON array into `out`.
///
/// # Safety
/// `client` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_plugins_json(
    client: *mut NidvueClient,
    out: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client.client.plugin_list().and_then(|plugins| {
            serde_json::to_string(&plugins)
                .map_err(|e| nidvue_core::NidvueError::Internal(format!("plugins json: {e}")))
        })
    })) {
        Ok(Ok(json)) => json_out(out, json),
        Ok(Err(_)) | Err(_) => NIDVUE_ERR,
    }
}

/// Run one sync pass for a single pair; the report JSON goes into `out`.
///
/// # Safety
/// `client` and `name` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_sync_pair_now(
    client: *mut NidvueClient,
    name: *const c_char,
    out: *mut *mut c_char,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    let Some(name) = cstr(name) else {
        set_err(err, "name is required");
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client
            .runtime
            .block_on(client.client.sync_pair_now(name))
            .and_then(|report| {
                serde_json::to_string(&report_json(&report))
                    .map_err(|e| nidvue_core::NidvueError::Internal(format!("report json: {e}")))
            })
    })) {
        Ok(Ok(json)) => json_out(out, json),
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_sync_pair_now");
            NIDVUE_PANIC
        }
    }
}

/// Sync pairs as a JSON array into `out`.
///
/// # Safety
/// `client` must be valid for the call; `out` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_sync_pairs_json(
    client: *mut NidvueClient,
    out: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        return NIDVUE_INVALID;
    };
    match serde_json::to_string(&client.client.sync_pairs()) {
        Ok(json) => json_out(out, json),
        Err(_) => NIDVUE_ERR,
    }
}

/// Add a sync pair, persist it, and write the new list into `out`.
///
/// # Safety
/// `client`, `name`, `local` and `remote` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_sync_pair_add(
    client: *mut NidvueClient,
    name: *const c_char,
    local: *const c_char,
    remote: *const c_char,
    out: *mut *mut c_char,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    let (Some(name), Some(local), Some(remote)) = (cstr(name), cstr(local), cstr(remote)) else {
        set_err(err, "name, local and remote are required");
        return NIDVUE_INVALID;
    };
    let pair = SyncPair {
        name: name.to_string(),
        local: std::path::PathBuf::from(local),
        remote: remote.to_string(),
        paused: false,
        include: Vec::new(),
        exclude: Vec::new(),
        conflict_policy: None,
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client.client.add_sync_pair(pair)?;
        serde_json::to_string(&client.client.sync_pairs())
            .map_err(|e| nidvue_core::NidvueError::Internal(format!("pairs json: {e}")))
    })) {
        Ok(Ok(json)) => json_out(out, json),
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_sync_pair_add");
            NIDVUE_PANIC
        }
    }
}

/// Remove a sync pair by name and write the remaining list into `out`.
///
/// # Safety
/// `client` and `name` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_sync_pair_remove(
    client: *mut NidvueClient,
    name: *const c_char,
    out: *mut *mut c_char,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    let Some(name) = cstr(name) else {
        set_err(err, "name is required");
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client.client.remove_sync_pair(name)?;
        serde_json::to_string(&client.client.sync_pairs())
            .map_err(|e| nidvue_core::NidvueError::Internal(format!("pairs json: {e}")))
    })) {
        Ok(Ok(json)) => json_out(out, json),
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_sync_pair_remove");
            NIDVUE_PANIC
        }
    }
}

/// Pause or resume one sync pair and write the new list into `out`.
///
/// # Safety
/// `client` and `name` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_sync_pair_set_paused(
    client: *mut NidvueClient,
    name: *const c_char,
    paused: i32,
    out: *mut *mut c_char,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    let Some(name) = cstr(name) else {
        set_err(err, "name is required");
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client.client.set_sync_pair_paused(name, paused != 0)?;
        serde_json::to_string(&client.client.sync_pairs())
            .map_err(|e| nidvue_core::NidvueError::Internal(format!("pairs json: {e}")))
    })) {
        Ok(Ok(json)) => json_out(out, json),
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_sync_pair_set_paused");
            NIDVUE_PANIC
        }
    }
}

/// Pause or resume every sync pair.
///
/// # Safety
/// `client` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_set_paused(
    client: *mut NidvueClient,
    paused: i32,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| client.client.set_paused(paused != 0))) {
        Ok(Ok(())) => NIDVUE_OK,
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_set_paused");
            NIDVUE_PANIC
        }
    }
}

/// Set the aggregate bandwidth cap in bytes/second (0 = unlimited).
///
/// # Safety
/// `client` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_set_bandwidth_limit(
    client: *mut NidvueClient,
    bytes_per_second: u64,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client.client.set_bandwidth_limit_bps(bytes_per_second)
    })) {
        Ok(Ok(())) => NIDVUE_OK,
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_set_bandwidth_limit");
            NIDVUE_PANIC
        }
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
                serde_json::to_string(&reports.iter().map(report_json).collect::<Vec<_>>())
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

/// Replace one pair's include and exclude globs.
///
/// Each argument is a comma-separated glob list; an empty string clears it.
/// The new pair list goes into `*out`.
///
/// # Safety
/// `client`, `name`, `include` and `exclude` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_sync_pair_set_filters(
    client: *mut NidvueClient,
    name: *const c_char,
    include: *const c_char,
    exclude: *const c_char,
    out: *mut *mut c_char,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    let Some(name) = cstr(name) else {
        set_err(err, "name is required");
        return NIDVUE_INVALID;
    };
    let split = |text: Option<&str>| -> Vec<String> {
        text.unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|glob| !glob.is_empty())
            .map(str::to_string)
            .collect()
    };
    let include = split(cstr(include));
    let exclude = split(cstr(exclude));
    match catch_unwind(AssertUnwindSafe(|| {
        client
            .client
            .set_sync_pair_filters(name, include, exclude)?;
        serde_json::to_string(&client.client.sync_pairs())
            .map_err(|e| nidvue_core::NidvueError::Internal(format!("pairs json: {e}")))
    })) {
        Ok(Ok(json)) => json_out(out, json),
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_sync_pair_set_filters");
            NIDVUE_PANIC
        }
    }
}

/// Read one configuration value by dotted key into `*out` as JSON.
///
/// # Safety
/// `client` and `key` must be valid for the call; `out` a valid out-pointer.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_config_get(
    client: *mut NidvueClient,
    key: *const c_char,
    out: *mut *mut c_char,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    let Some(key) = cstr(key) else {
        set_err(err, "key is required");
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        client
            .client
            .config_get(key)
            .and_then(|value| {
                serde_json::to_string(&value)
                    .map_err(|e| nidvue_core::NidvueError::Internal(format!("config json: {e}")))
            })
    })) {
        Ok(Ok(json)) => json_out(out, json),
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_config_get");
            NIDVUE_PANIC
        }
    }
}

/// Write one configuration value by dotted key and persist it.
///
/// The value is JSON when it parses (lists, numbers, booleans, quoted strings)
/// and a plain string otherwise.
///
/// # Safety
/// `client`, `key` and `value` must be valid for the call.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_config_set(
    client: *mut NidvueClient,
    key: *const c_char,
    value: *const c_char,
    err: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        set_err(err, "client is null");
        return NIDVUE_INVALID;
    };
    let (Some(key), Some(value)) = (cstr(key), cstr(value)) else {
        set_err(err, "key and value are required");
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| client.client.config_set(key, value))) {
        Ok(Ok(_)) => NIDVUE_OK,
        Ok(Err(e)) => {
            set_err(err, e.to_string());
            NIDVUE_ERR
        }
        Err(_) => {
            set_err(err, "panic contained in nidvue_client_config_set");
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

/// Transfers seen so far as a JSON array, oldest first.
///
/// # Safety
/// `client` must be valid; `out` must be a valid out-pointer.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_transfers_json(
    client: *mut NidvueClient,
    out: *mut *mut c_char,
) -> i32 {
    let Some(client) = client.as_ref() else {
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| {
        serde_json::to_string(&client.client.transfers())
            .map_err(|e| nidvue_core::NidvueError::Internal(format!("transfers json: {e}")))
    })) {
        Ok(Ok(json)) => json_out(out, json),
        Ok(Err(_)) | Err(_) => NIDVUE_ERR,
    }
}

/// Forget every recorded transfer row.
///
/// # Safety
/// `client` must be valid.
#[no_mangle]
pub unsafe extern "C" fn nidvue_client_clear_transfers(client: *mut NidvueClient) -> i32 {
    let Some(client) = client.as_ref() else {
        return NIDVUE_INVALID;
    };
    match catch_unwind(AssertUnwindSafe(|| client.client.clear_transfers())) {
        Ok(()) => NIDVUE_OK,
        Err(_) => NIDVUE_PANIC,
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
    fn sync_pairs_round_trip_through_the_c_abi() {
        let dir = tempfile::tempdir().unwrap();
        let state = CString::new(dir.path().to_str().unwrap()).unwrap();
        let local = dir.path().join("docs");
        let name = CString::new("docs").unwrap();
        let local_c = CString::new(local.to_str().unwrap()).unwrap();
        let remote = CString::new("/Archive/Documents").unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        // SAFETY: every pointer is valid for the duration of the test.
        unsafe {
            let client = nidvue_client_new(state.as_ptr(), &mut err);
            assert!(!client.is_null());

            let mut out: *mut c_char = ptr::null_mut();
            assert_eq!(nidvue_client_sync_pairs_json(client, &mut out), NIDVUE_OK);
            assert_eq!(take(out), "[]");

            let mut out: *mut c_char = ptr::null_mut();
            assert_eq!(
                nidvue_client_sync_pair_add(
                    client,
                    name.as_ptr(),
                    local_c.as_ptr(),
                    remote.as_ptr(),
                    &mut out,
                    &mut err
                ),
                NIDVUE_OK
            );
            let added = take(out);
            assert!(added.contains("\"name\":\"docs\""), "{added}");
            assert!(added.contains("\"paused\":false"), "{added}");

            let mut out: *mut c_char = ptr::null_mut();
            assert_eq!(
                nidvue_client_sync_pair_set_paused(client, name.as_ptr(), 1, &mut out, &mut err),
                NIDVUE_OK
            );
            assert!(take(out).contains("\"paused\":true"));

            let mut out: *mut c_char = ptr::null_mut();
            assert_eq!(
                nidvue_client_sync_pair_remove(client, name.as_ptr(), &mut out, &mut err),
                NIDVUE_OK
            );
            assert_eq!(take(out), "[]");
            nidvue_client_free(client);
        }
        assert!(!local.exists() || local.is_dir());
    }

    #[test]
    fn sync_pair_errors_reach_the_caller() {
        let dir = tempfile::tempdir().unwrap();
        let state = CString::new(dir.path().to_str().unwrap()).unwrap();
        let name = CString::new("ghost").unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        // SAFETY: every pointer is valid for the duration of the test.
        unsafe {
            let client = nidvue_client_new(state.as_ptr(), &mut err);
            let mut out: *mut c_char = ptr::null_mut();
            assert_eq!(
                nidvue_client_sync_pair_remove(client, name.as_ptr(), &mut out, &mut err),
                NIDVUE_ERR
            );
            assert!(!err.is_null(), "the error message is set");
            let message = take(err);
            assert!(message.contains("ghost"), "{message}");
            nidvue_client_free(client);
        }
    }

    #[test]
    fn a_sync_pass_with_no_pairs_is_empty_and_a_named_pass_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let state = CString::new(dir.path().to_str().unwrap()).unwrap();
        let missing = CString::new("ghost").unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        // SAFETY: every pointer is valid for the duration of the test.
        unsafe {
            let client = nidvue_client_new(state.as_ptr(), &mut err);
            let mut out: *mut c_char = ptr::null_mut();
            assert_eq!(
                nidvue_client_sync_now(client, &mut out, &mut err),
                NIDVUE_OK
            );
            assert_eq!(take(out), "[]");

            let mut out: *mut c_char = ptr::null_mut();
            assert_eq!(
                nidvue_client_sync_pair_now(client, missing.as_ptr(), &mut out, &mut err),
                NIDVUE_ERR
            );
            assert!(!err.is_null());
            assert!(take(err).contains("ghost"));
            nidvue_client_free(client);
        }
    }

    #[test]
    fn cache_limit_is_set_and_zero_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let state = CString::new(dir.path().to_str().unwrap()).unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        // SAFETY: every pointer is valid for the duration of the test.
        unsafe {
            let client = nidvue_client_new(state.as_ptr(), &mut err);
            assert_eq!(
                nidvue_client_set_cache_limit(client, 4 * 1024 * 1024 * 1024, &mut err),
                NIDVUE_OK
            );
            assert!(err.is_null());
            let stored = nidvue_core::config::AppConfig::load_or_default(dir.path()).unwrap();
            assert_eq!(stored.cache.max_size_bytes, 4 * 1024 * 1024 * 1024);

            assert_eq!(
                nidvue_client_set_cache_limit(client, 0, &mut err),
                NIDVUE_ERR
            );
            assert!(!err.is_null());
            take(err);
            nidvue_client_free(client);
        }
    }

    #[test]
    fn plugins_list_is_empty_for_a_fresh_state_directory() {
        let dir = tempfile::tempdir().unwrap();
        let state = CString::new(dir.path().to_str().unwrap()).unwrap();
        let mut err: *mut c_char = ptr::null_mut();
        // SAFETY: every pointer is valid for the duration of the test.
        unsafe {
            let client = nidvue_client_new(state.as_ptr(), &mut err);
            let mut out: *mut c_char = ptr::null_mut();
            assert_eq!(nidvue_client_plugins_json(client, &mut out), NIDVUE_OK);
            assert_eq!(take(out), "[]");
            nidvue_client_free(client);
        }
    }

    struct Collector {
        seen: Mutex<Vec<String>>,
    }

    extern "C" fn collect(json: *const c_char, userdata: *mut c_void) {
        // SAFETY: the callback is only installed for the test's collector.
        let text = unsafe { CStr::from_ptr(json) }
            .to_string_lossy()
            .into_owned();
        // SAFETY:  points at a live  for the whole test.
        let collector = unsafe { &*(userdata as *const Collector) };
        collector.seen.lock().unwrap().push(text);
    }

    #[test]
    fn events_reach_the_c_callback_as_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = CString::new(dir.path().to_str().unwrap()).unwrap();
        let collector = Box::new(Collector {
            seen: Mutex::new(Vec::new()),
        });
        let mut err: *mut c_char = ptr::null_mut();
        // SAFETY: the collector outlives the subscription for the test.
        unsafe {
            let client = nidvue_client_new(state.as_ptr(), &mut err);
            assert_eq!(
                nidvue_client_subscribe(
                    client,
                    Some(collect),
                    &*collector as *const Collector as *mut c_void
                ),
                NIDVUE_OK
            );
            assert_eq!(nidvue_client_set_paused(client, 1, &mut err), NIDVUE_OK);

            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while collector.seen.lock().unwrap().is_empty() && std::time::Instant::now() < deadline
            {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            let seen = collector.seen.lock().unwrap().clone();
            assert!(!seen.is_empty(), "the callback received nothing");
            assert!(
                seen.iter().any(|json| json.contains("\"type\"")),
                "events are JSON: {seen:?}"
            );
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
