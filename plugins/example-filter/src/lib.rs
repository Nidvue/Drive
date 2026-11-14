// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Example plugin: scope a sync to one folder and pick a conflict policy.
//!
//! Declares read access to `/Archive/Documents` only; the host refuses any
//! other path even if the plugin asks.

#![no_std]

use core::ptr::{addr_of, addr_of_mut};
use core::slice;

const OUT_CAP: usize = 4096;
static mut OUT: [u8; OUT_CAP] = [0; OUT_CAP];
static mut NEXT: usize = 0;
const IN_CAP: usize = 256 * 1024;
static mut IN: [u8; IN_CAP] = [0; IN_CAP];

const ROOT: &[u8] = b"/Archive/Documents";

#[no_mangle]
pub extern "C" fn nidvue_plugin_init() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn nidvue_plugin_alloc(len: u32) -> u32 {
    unsafe {
        let next = addr_of_mut!(NEXT);
        if *next + len as usize > IN_CAP {
            *next = 0;
        }
        let base = addr_of!(IN) as *const u8;
        let ptr = base.add(*next);
        *next += len as usize;
        ptr as u32
    }
}

#[no_mangle]
pub extern "C" fn nidvue_plugin_call(hook: u32, ptr: u32, len: u32) -> u64 {
    let request = unsafe { slice::from_raw_parts(ptr as *const u8, len as usize) };
    let response: &[u8] = match hook {
        // path_filter
        0 => {
            let path = field(request, b"\"path\":\"").unwrap_or(&[]);
            if path == ROOT || starts_with(path, b"/Archive/Documents/") {
                b"{\"include\":true}"
            } else {
                b"{\"include\":false}"
            }
        }
        // conflict_resolve: documents are edited locally, so local wins and the
        // remote version is kept as a conflict copy.
        4 => b"{\"winner\":\"local\"}",
        _ => b"{}",
    };
    unsafe {
        let out = addr_of_mut!(OUT) as *mut u8;
        let n = response.len().min(OUT_CAP);
        core::ptr::copy_nonoverlapping(response.as_ptr(), out, n);
        ((out as u64) << 32) | n as u64
    }
}

fn field<'a>(json: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let start = find(json, key)? + key.len();
    let mut end = start;
    while end < json.len() && json[end] != b'"' {
        end += 1;
    }
    Some(&json[start..end])
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

fn starts_with(haystack: &[u8], prefix: &[u8]) -> bool {
    haystack.len() >= prefix.len() && &haystack[..prefix.len()] == prefix
}

/// A panic aborts this module; the host contains the trap and reports it.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
