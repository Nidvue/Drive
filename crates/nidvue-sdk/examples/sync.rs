// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Run one sync pass and print the plan's outcome.

use nidvue_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = std::env::var("NIDVUE_STATE").unwrap_or_else(|_| "/tmp/nidvue-state".into());
    let client = Client::open(&state).await?;
    if client.sync_pairs().is_empty() {
        println!("no sync pairs configured; add one to config.toml");
        return Ok(());
    }
    for report in client.sync_now().await? {
        println!(
            "scanned {} planned {} applied {} conflicts {} failed {}",
            report.scanned,
            report.planned,
            report.applied,
            report.conflicts,
            report.failed.len()
        );
        for (path, error) in &report.failed {
            eprintln!("  {path}: {error}");
        }
    }
    Ok(())
}
