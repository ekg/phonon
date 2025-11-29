//! Test rendering x.ph file to actual audio output
//!
//! This test verifies that the user's x.ph code:
//! 1. Parses successfully with the compositional parser
//! 2. Compiles to a graph
//! 3. Renders audio to a WAV file
//! 4. Produces non-silent audio

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use std::fs;

#[test]
fn test_render_x_ph_to_wav() {
    // Read the user's x.ph file
    let x_ph_path = "x.ph";

    if !std::path::Path::new(x_ph_path).exists() {
        println!("⚠️  x.ph not found, skipping test");
        return;
    }

    let code = fs::read_to_string(x_ph_path).expect("Failed to read x.ph");

    println!("📄 Contents of x.ph:");
    println!("{}", code);
    println!();

    // Parse
    println!("🔍 Parsing...");
    let parse_result = parse_program(&code);

    if let Err(e) = parse_result {
        panic!("❌ Parse failed: {:?}", e);
    }

    let (rest, statements) = parse_result.unwrap();
    println!("✅ Parsed successfully!");
    println!("   Statements: {}", statements.len());

    if !rest.trim().is_empty() {
        println!("⚠️  Unparsed remainder: '{}'", rest);
    }

    // Compile
    println!();
    println!("🔨 Compiling...");
    let compile_result = compile_program(statements, 44100.0, None);

    if let Err(ref e) = compile_result {
        panic!("❌ Compile failed: {}", e);
    }

    let mut graph = compile_result.unwrap();
    println!("✅ Compiled successfully!");

    // Set tempo (2 cycles per second = 120 BPM)
    graph.set_cps(2.0);

    // Render 2 seconds of audio
    println!();
    println!("🎵 Rendering 2 seconds of audio...");
    let sample_rate = 44100;
    let duration_samples = sample_rate * 2;
    let buffer = graph.render(duration_samples);

    println!("   Rendered {} samples", buffer.len());

    // Verify audio is not silent
    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let rms = (buffer.iter().map(|s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("   Max amplitude: {:.6}", max_amplitude);
    println!("   RMS level: {:.6}", rms);

    assert!(
        max_amplitude > 0.001,
        "Audio should not be silent! Max amplitude: {}",
        max_amplitude
    );
    assert!(
        rms > 0.0001,
        "Audio RMS should be above noise floor! RMS: {}",
        rms
    );

    // Write to WAV file for inspection
    println!();
    println!("💾 Writing to x_ph_output.wav...");

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create("x_ph_output.wav", spec).unwrap();

    for &sample in &buffer {
        let amplitude = (sample * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).unwrap();
    }

    writer.finalize().unwrap();

    println!("✅ SUCCESS! x.ph rendered to x_ph_output.wav");
    println!();
    println!("🎧 Listen to the output:");
    println!("   play x_ph_output.wav");
}

#[test]
fn test_render_complex_compositional_example() {
    // Test a complex example that exercises all features
    let code = r#"
        tempo: 0.5

        ~kick $ s "bd*4"
        ~snare $ s "~ sn ~ sn"

        ~cutoffs $ "500 1000 2000 4000" $ fast 4
        ~filtered $ ~kick # lpf ~cutoffs 0.8

        ~bass $ saw 55 # lpf 400 0.6

        out $ ~filtered * 0.5 + ~snare * 0.4 + ~bass * 0.3
    "#;

    println!("🔍 Parsing complex example...");
    let (_, statements) = parse_program(code).expect("Failed to parse");

    println!("🔨 Compiling...");
    let mut graph = compile_program(statements, 44100.0, None).expect("Failed to compile");
    graph.set_cps(2.0);

    println!("🎵 Rendering...");
    let buffer = graph.render(88200); // 2 seconds

    println!("   Rendered {} samples", buffer.len());

    let max_amplitude = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let rms = (buffer.iter().map(|s| s * s).sum::<f32>() / buffer.len() as f32).sqrt();

    println!("   Max amplitude: {:.6}", max_amplitude);
    println!("   RMS level: {:.6}", rms);

    // Write to WAV
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create("complex_example_output.wav", spec).unwrap();

    for &sample in &buffer {
        let amplitude = (sample * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).unwrap();
    }

    writer.finalize().unwrap();

    println!("✅ Written to complex_example_output.wav");

    assert!(max_amplitude > 0.001, "Should have audio");
    assert!(rms > 0.0001, "RMS should be above noise floor");
}
