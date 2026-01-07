//! List VST3 Plugins
//!
//! Simple utility to scan for and list all VST3 plugins on the system.

#[cfg(feature = "vst3")]
fn main() {
    use phonon::plugin_host::{PluginRegistry, RealPluginScanner};

    println!("Phonon VST3 Plugin Scanner");
    println!("==========================\n");

    // Try the rack scanner first
    println!("Scanning with rack crate...\n");

    match RealPluginScanner::new() {
        Ok(scanner) => match scanner.scan() {
            Ok(plugins) => {
                if plugins.is_empty() {
                    println!("No VST3 plugins found in system paths.");
                    println!("\nSearched paths:");
                    println!("  - ~/.vst3");
                    println!("  - /usr/lib/vst3");
                    println!("  - /usr/local/lib/vst3");
                } else {
                    println!("Found {} VST3 plugin(s):\n", plugins.len());
                    for (i, plugin) in plugins.iter().enumerate() {
                        println!(
                            "{}. {} by {} ({:?})",
                            i + 1,
                            plugin.name,
                            plugin.manufacturer,
                            plugin.plugin_type
                        );
                        println!("   Path: {}", plugin.path.display());
                        println!("   ID: {}", plugin.unique_id);
                        println!();
                    }
                }
            }
            Err(e) => {
                eprintln!("Error scanning for plugins: {}", e);
            }
        },
        Err(e) => {
            eprintln!("Error creating scanner: {}", e);
        }
    }

    // Also show what the registry finds with basic filesystem scan
    println!("\n---------------------------------");
    println!("Basic filesystem scan:\n");

    let mut registry = PluginRegistry::new();
    match registry.scan() {
        Ok(count) => {
            if count == 0 {
                println!("No plugins found with filesystem scan.");
            } else {
                println!("Found {} plugin bundle(s):\n", count);
                for plugin in registry.list() {
                    println!(
                        "  - {} [{:?}] at {}",
                        plugin.id.name, plugin.id.format, plugin.path
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("Error scanning: {}", e);
        }
    }
}

#[cfg(not(feature = "vst3"))]
fn main() {
    println!("VST3 support not enabled. Build with --features vst3");
}
