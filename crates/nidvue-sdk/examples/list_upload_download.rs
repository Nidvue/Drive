// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! List a directory, upload a file, download it back and verify the bytes.

use nidvue_sdk::{Client, RemotePath};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = std::env::var("NIDVUE_STATE").unwrap_or_else(|_| "/tmp/nidvue-state".into());
    let client = Client::open(&state).await?;
    if !client.is_logged_in().await {
        println!("run the authenticate example first");
        return Ok(());
    }
    let dir = RemotePath::parse("/")?;
    let listing = client.list(&dir).await?;
    println!(
        "{} files, {} folders at the root",
        listing.files.len(),
        listing.folders.len()
    );

    let local = std::path::Path::new("/tmp/nidvue-example.txt");
    std::fs::write(local, b"hello from the Nidvue SDK\n")?;
    let remote = RemotePath::parse("/nidvue-example.txt")?;
    let entry = client.upload(&remote, local).await?;
    println!("uploaded {} bytes", entry.size);

    let back = std::path::Path::new("/tmp/nidvue-example-back.txt");
    let size = client.download(&remote, back).await?;
    println!("downloaded {size} bytes");
    assert_eq!(std::fs::read(local)?, std::fs::read(back)?);
    Ok(())
}
