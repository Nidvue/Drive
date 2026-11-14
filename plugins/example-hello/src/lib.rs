// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Example plugin: a path filter.
//!
//! Skips build caches and temporary files. It declares no filesystem or network
//! capability, so it cannot reach anything beyond the payload the host hands it.

#![no_std]

use core::ptr::{addr_of, addr_of_mut};
use core::slice;

#[link(wasm_import_module = "nidvue")]
extern "C" {
    fn log(level: i32, ptr: i32, len: i32);
}

const OUT_CAP: usize = 4096;
static mut OUT: [u8; OUT_CAP] = [0; OUT_CAP];
static mut NEXT: usize = 0;
const IN_CAP: usize = 256 * 1024;
static mut IN: [u8; IN_CAP] = [0; IN_CAP];

/// ABI: 0 = ok.
#[no_mangle]
pub extern "C" fn nidvue_plugin_init() -> i32 {
    0
}

/// ABI: hand the host a buffer to write the request into.
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

/// ABI: (ptr << 32) | len of the response.
#[no_mangle]
pub extern "C" fn nidvue_plugin_call(hook: u32, ptr: u32, len: u32) -> u64 {
    let request = unsafe { slice::from_raw_parts(ptr as *const u8, len as usize) };
    let response: &[u8] = match hook {
        0 => path_filter(request),
        5 => {
            log_line(b"hello: saw an event");
            b"{}"
        }
        _ => b"{}",
    };
    unsafe {
        let out = addr_of_mut!(OUT) as *mut u8;
        let n = response.len().min(OUT_CAP);
        core::ptr::copy_nonoverlapping(response.as_ptr(), out, n);
        ((out as u64) << 32) | n as u64
    }
}

fn path_filter(request: &[u8]) -> &'static [u8] {
    let path = field(request, b"\"path\":\"").unwrap_or(&[]);
    let skip = contains(path, b"/.git/")
        || contains(path, b"/node_modules/")
        || contains(path, b"/target/")
        || ends_with(path, b".tmp")
        || ends_with(path, b".nidvue-tmp");
    if skip {
        b"{\"include\":false}"
    } else {
        b"{\"include\":true}"
    }
}

/// The value of a JSON string field, without pulling in a parser: this plugin
/// only ever reads the one field the host sends it.
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

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    find(haystack, needle).is_some()
}

fn ends_with(haystack: &[u8], suffix: &[u8]) -> bool {
    haystack.len() >= suffix.len() && &haystack[haystack.len() - suffix.len()..] == suffix
}

fn log_line(message: &[u8]) {
    unsafe { log(1, message.as_ptr() as i32, message.len() as i32) }
}

/// A panic aborts this module; the host contains the trap and reports it.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
