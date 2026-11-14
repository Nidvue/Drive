// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Subscribe to typed engine events.

use nidvue_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = std::env::var("NIDVUE_STATE").unwrap_or_else(|_| "/tmp/nidvue-state".into());
    let client = Client::open(&state).await?;
    let mut rx = client.subscribe();
    println!("listening for events (Ctrl-C to stop)");
    loop {
        match rx.recv().await {
            Ok(envelope) => println!(
                "{} {} {}",
                envelope.ts_ms,
                envelope.event.kind(),
                serde_json::to_string(&envelope.event)?
            ),
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                eprintln!("dropped {n} events");
            }
            Err(_) => break,
        }
    }
    Ok(())
}
