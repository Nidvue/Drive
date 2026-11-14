// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! Load the example plugin and call its path filter.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut host = nidvue_plugin::PluginHost::new()?;
    let dir = std::path::Path::new("plugins/example-hello");
    if !dir.exists() {
        println!("run from the repository root after `cargo xtask plugins`");
        return Ok(());
    }
    let id = host.load(dir)?;
    let plugin = host.plugin_mut(&id).ok_or("plugin vanished")?;
    println!(
        "{} says {}",
        id,
        plugin.call_hook("path_filter", r#"{"path":"/Archive/project/target/x.o"}"#)?
    );
    Ok(())
}
