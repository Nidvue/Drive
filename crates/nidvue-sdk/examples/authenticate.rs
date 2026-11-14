// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Authenticate with a Personal Login Token.
//!
//! ```sh
//! NIDVUE_TOKEN='<paste the token>' NIDVUE_STATE=/tmp/nidvue cargo run -p nidvue-sdk --example authenticate
//! ```

use nidvue_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = std::env::var("NIDVUE_STATE").unwrap_or_else(|_| "/tmp/nidvue-state".into());
    let token = std::env::var("NIDVUE_TOKEN").map_err(|_| "set NIDVUE_TOKEN to a Personal Login Token")?;
    let client = Client::open(&state).await?;
    let username = client.login(&token).await?;
    println!("logged in as {username}");
    println!("{}", serde_json::to_string_pretty(&client.status().await?)?);
    Ok(())
}
