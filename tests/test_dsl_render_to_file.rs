use hound::{SampleFormat, WavSpec, WavWriter};
/// Test DSL rendering to file to verify DslCompiler works end-to-end
use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

#[test]
fn test_dsl_render_to_wav() {
    let dsl_code = r#"
tempo: 0.5
out: s "bd" * 0.8
"#;

    let (_, statements) = parse_dsl(dsl_code).expect("Failed to parse DSL");
    println!("Parsed {} statements", statements.len());

    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 1 cycle (2 seconds at 0.5 cps)
    let duration = 1.0 / graph.get_cps(); // 1 cycle = 2 seconds
    let total_samples = (duration * 44100.0) as usize;

    println!("Rendering {} samples ({} seconds)", total_samples, duration);

    let buffer = graph.render(total_samples);

    // Check peak
    let peak = buffer.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    println!("Peak: {:.6}", peak);

    // Write to WAV
    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer =
        WavWriter::create("/tmp/test_dsl_render.wav", spec).expect("Failed to create WAV");

    for &sample in &buffer {
        let sample_i16 = (sample * 32767.0) as i16;
        writer
            .write_sample(sample_i16)
            .expect("Failed to write sample");
    }

    writer.finalize().expect("Failed to finalize WAV");

    println!("✅ Wrote WAV to /tmp/test_dsl_render.wav");

    // Verify audio was produced
    assert!(peak > 0.005, "Peak too low: {:.6}", peak);
}
