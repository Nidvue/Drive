// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Mount the account at a local path and read a file through it.

use nidvue_mount::MountOptions;
use nidvue_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = std::env::var("NIDVUE_STATE").unwrap_or_else(|_| "/tmp/nidvue-state".into());
    let mountpoint = std::env::var("NIDVUE_MOUNT").unwrap_or_else(|_| "/tmp/nidvue-mnt".into());
    let client = Client::open(&state).await?;
    let handle = client.mount(std::path::Path::new(&mountpoint), MountOptions::default())?;
    println!("mounted at {mountpoint}; listing the root");
    for entry in std::fs::read_dir(&mountpoint)? {
        println!("  {}", entry?.file_name().to_string_lossy());
    }
    println!("press enter to unmount");
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    drop(handle);
    Ok(())
}
