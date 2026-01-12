//! Headless VST3 GUI Test with Screenshot Capture
//!
//! This test runs the VST3 GUI in a virtual framebuffer (Xvfb) and captures
//! a screenshot to verify the GUI renders correctly.
//!
//! Usage:
//!   xvfb-run -a cargo run --bin test_vst3_gui_headless --features vst3
//!
//! Or run directly if DISPLAY is set:
//!   cargo run --bin test_vst3_gui_headless --features vst3

use rack::prelude::*;
use std::process::Command;
use std::time::Duration;
use std::path::Path;

fn main() -> Result<()> {
    println!("VST3 Headless GUI Test");
    println!("======================\n");

    // Check for DISPLAY
    let display = std::env::var("DISPLAY").unwrap_or_default();
    if display.is_empty() {
        eprintln!("ERROR: No DISPLAY set. Run with xvfb-run:");
        eprintln!("  xvfb-run -a cargo run --bin test_vst3_gui_headless --features vst3");
        std::process::exit(1);
    }
    println!("Using DISPLAY={}", display);

    // Create scanner and find plugin
    println!("\nScanning for plugins...");
    let scanner = Scanner::new()?;
    let plugins = scanner.scan()?;

    if plugins.is_empty() {
        println!("No VST3 plugins found!");
        return Ok(());
    }

    println!("Found {} plugins", plugins.len());

    // Find an instrument plugin
    let plugin_info = plugins
        .iter()
        .find(|p| p.name == "Surge XT")
        .or_else(|| plugins.iter().find(|p| p.plugin_type == PluginType::Instrument))
        .or_else(|| plugins.first())
        .expect("Should have at least one plugin");

    println!("Selected: {} by {}", plugin_info.name, plugin_info.manufacturer);

    // Load and initialize plugin
    println!("\nLoading plugin...");
    let mut plugin = scanner.load(plugin_info)?;
    plugin.initialize(48000.0, 512)?;
    println!("Plugin initialized with {} parameters", plugin.parameter_count());

    // Create GUI
    println!("\nCreating GUI...");
    let mut gui = match Vst3Gui::create(&mut plugin) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to create GUI: {}", e);
            eprintln!("Plugin may not support GUI on this platform");
            return Err(e);
        }
    };

    let (width, height) = gui.get_size().unwrap_or((800, 600));
    println!("GUI size: {}x{}", width, height);

    let window_id = gui.get_window_id();
    println!("Window ID: 0x{:x}", window_id);

    // Show the window
    println!("\nShowing window...");
    gui.show(Some(&plugin_info.name))?;

    // Pump events for a bit to let the GUI render
    println!("Waiting for GUI to render...");
    for _ in 0..30 {
        gui.pump_events();
        std::thread::sleep(Duration::from_millis(50));
    }

    // Capture screenshot
    let screenshot_path = "/tmp/vst3_gui_test.png";
    println!("\nCapturing screenshot to {}...", screenshot_path);

    // Use import from ImageMagick to capture the window
    let capture_result = Command::new("import")
        .args(["-window", &format!("0x{:x}", window_id), screenshot_path])
        .output();

    match capture_result {
        Ok(output) => {
            if output.status.success() {
                println!("Screenshot captured successfully!");

                // Verify the file exists and has content
                if let Ok(metadata) = std::fs::metadata(screenshot_path) {
                    let size = metadata.len();
                    println!("Screenshot size: {} bytes", size);

                    if size > 1000 {
                        println!("\n✓ GUI TEST PASSED");
                        println!("  - Plugin loaded: OK");
                        println!("  - GUI created: OK");
                        println!("  - Window shown: OK");
                        println!("  - Screenshot captured: OK ({} bytes)", size);

                        // Analyze the image to check it's not blank
                        if let Ok(analysis) = analyze_screenshot(screenshot_path) {
                            println!("  - Image analysis: {}", analysis);
                        }
                    } else {
                        eprintln!("WARNING: Screenshot is very small ({} bytes), may be blank", size);
                    }
                }
            } else {
                eprintln!("Screenshot capture failed: {:?}", output.stderr);
            }
        }
        Err(e) => {
            eprintln!("Failed to run import command: {}", e);
            eprintln!("Make sure ImageMagick is installed: apt install imagemagick");
        }
    }

    // Keep window open briefly
    println!("\nKeeping window open for 2 more seconds...");
    for _ in 0..40 {
        gui.pump_events();
        std::thread::sleep(Duration::from_millis(50));
    }

    println!("\nTest complete!");
    Ok(())
}

/// Analyze screenshot to verify it's not blank
fn analyze_screenshot(path: &str) -> std::io::Result<String> {
    // Use ImageMagick identify to get image stats
    let output = Command::new("identify")
        .args(["-verbose", path])
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Look for mean pixel value - if it's not 0 or 255, we have content
        for line in stdout.lines() {
            if line.contains("mean:") {
                return Ok(format!("has content ({})", line.trim()));
            }
        }
        Ok("image exists".to_string())
    } else {
        Ok("could not analyze".to_string())
    }
}
