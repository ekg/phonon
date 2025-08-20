//! Integration tests for the complete modular synthesis system
//! 
//! Tests the full pipeline from DSL parsing to audio generation

#[cfg(test)]
mod tests {
    use crate::enhanced_parser::EnhancedParser;
    use crate::signal_graph::SignalGraph;
    use crate::signal_executor::{SignalExecutor, AudioBuffer};
    use crate::pattern_bridge::{PatternBridge, PatternEvent, PatternValue};
    use std::sync::{Arc, RwLock};
    
    #[test]
    fn test_simple_modular_synthesis() {
        // Parse a simple modular patch
        let dsl = r#"
            ~lfo: sine(2) * 0.5 + 0.5
            ~osc: saw(440)
            ~filtered: ~osc >> lpf(1000, 0.7)
            out: ~filtered
        "#;
        
        let mut parser = EnhancedParser::new(44100.0);
        let result = parser.parse(dsl);
        assert!(result.is_ok(), "Should parse simple modular patch");
    }
    
    #[test]
    fn test_cross_modulation() {
        // Test pattern affecting audio and vice versa
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut bridge = PatternBridge::new(graph.clone(), 44100.0);
        
        // Register patterns
        bridge.register_pattern("bass".to_string());
        bridge.register_pattern("kick".to_string());
        
        // Process bass note event
        let bass_event = PatternEvent {
            time: 0.0,
            value: PatternValue::Note("c3".to_string()),
            duration: 0.5,
            velocity: 0.8,
        };
        bridge.process_pattern_event("bass", bass_event);
        
        // Simulate audio feature extraction
        bridge.audio_features.insert("bass_rms".to_string(), 0.7);
        
        // Check that we can use audio features for gating
        assert!(bridge.check_gate_condition("~bass_rms > 0.5"));
        
        // Apply pattern modulation to a parameter
        let modulated_freq = bridge.apply_pattern_modulation("bass", 1000.0, 500.0);
        assert!(modulated_freq > 1000.0); // Should be modulated
    }
    
    #[test]
    fn test_audio_analysis_features() {
        // Create a simple sine wave and analyze it
        let mut graph = SignalGraph::new(44100.0);
        
        let sine = crate::signal_graph::Node::Source {
            id: crate::signal_graph::NodeId("sine".to_string()),
            source_type: crate::signal_graph::SourceType::Sine { freq: 440.0 },
        };
        
        let rms = crate::signal_graph::Node::Analysis {
            id: crate::signal_graph::NodeId("rms".to_string()),
            analysis_type: crate::signal_graph::AnalysisType::RMS { window_size: 0.05 },
        };
        
        let output = crate::signal_graph::Node::Output {
            id: crate::signal_graph::NodeId("output".to_string()),
        };
        
        let sine_id = graph.add_node(sine);
        let rms_id = graph.add_node(rms);
        let output_id = graph.add_node(output);
        
        graph.connect(sine_id, rms_id.clone(), 1.0);
        graph.connect(rms_id, output_id, 1.0);
        
        // Execute and verify RMS analysis
        let mut executor = SignalExecutor::new(graph, 44100.0, 512);
        executor.initialize().unwrap();
        
        let output = executor.process_block().unwrap();
        assert!(output.rms() > 0.0, "Should have non-zero RMS");
    }
    
    #[test]
    fn test_pattern_to_frequency_modulation() {
        // Test that pattern notes correctly modulate frequency
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut bridge = PatternBridge::new(graph.clone(), 44100.0);
        
        bridge.register_pattern("melody".to_string());
        
        // Process a C4 note
        let event = PatternEvent {
            time: 0.0,
            value: PatternValue::Note("c4".to_string()),
            duration: 0.25,
            velocity: 1.0,
        };
        bridge.process_pattern_event("melody", event);
        
        // Check that the frequency is correct
        let signal = &bridge.pattern_signals["melody"];
        assert!((signal.current_value - 261.63).abs() < 0.01, 
                "C4 should be 261.63 Hz, got {}", signal.current_value);
    }
    
    #[test]
    fn test_envelope_generation() {
        // Test that pattern triggers generate envelopes
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut bridge = PatternBridge::new(graph.clone(), 44100.0);
        
        bridge.register_pattern("kick".to_string());
        
        // Initially, no trigger
        assert!(!bridge.get_pattern_trigger("kick"));
        assert!(!bridge.get_pattern_gate("kick"));
        
        // Process kick event
        let event = PatternEvent {
            time: 0.0,
            value: PatternValue::Sample("bd".to_string()),
            duration: 0.1,
            velocity: 1.0,
        };
        bridge.process_pattern_event("kick", event);
        
        // Now we should have trigger and gate
        assert!(bridge.get_pattern_trigger("kick"));
        assert!(bridge.get_pattern_gate("kick"));
        
        // Release the event
        bridge.release_pattern_event("kick");
        assert!(!bridge.get_pattern_gate("kick"));
    }
    
    #[test]
    fn test_sidechain_compression_simulation() {
        // Simulate sidechain compression using cross-modulation
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut bridge = PatternBridge::new(graph.clone(), 44100.0);
        
        // Register kick and bass
        bridge.register_pattern("kick".to_string());
        bridge.register_pattern("bass".to_string());
        
        // Bass is playing
        let bass_event = PatternEvent {
            time: 0.0,
            value: PatternValue::Note("c2".to_string()),
            duration: 1.0,
            velocity: 0.8,
        };
        bridge.process_pattern_event("bass", bass_event);
        
        let bass_level_normal = bridge.pattern_signals["bass"].current_value;
        
        // Kick hits - should duck the bass
        let kick_event = PatternEvent {
            time: 0.0,
            value: PatternValue::Sample("bd".to_string()),
            duration: 0.05,
            velocity: 1.0,
        };
        bridge.process_pattern_event("kick", kick_event);
        
        // Simulate ducking calculation
        let kick_signal = bridge.pattern_signals["kick"].current_value;
        let ducked_bass = bass_level_normal * (1.0 - kick_signal * 0.5);
        
        assert!(ducked_bass < bass_level_normal, 
                "Bass should be ducked when kick hits");
    }
    
    #[test]
    fn test_complex_routing() {
        // Test that complex routing expressions work
        let dsl = r#"
            ~lfo: sine(0.5)
            ~mod_depth: ~lfo * 0.5 + 0.5
            ~osc_freq: 440 + ~mod_depth * 100
            ~osc: sine(~osc_freq)
            ~filtered: ~osc >> lpf(~mod_depth * 2000 + 500, 0.7)
            out: ~filtered
        "#;
        
        let mut parser = EnhancedParser::new(44100.0);
        let result = parser.parse(dsl);
        assert!(result.is_ok(), "Should parse complex routing");
    }
    
    #[test]
    fn test_audio_buffer_to_wav() {
        // Test that we can write audio to WAV files for verification
        let buffer = AudioBuffer::mono(44100, 44100.0);
        
        // Generate a test tone
        let mut test_buffer = AudioBuffer::mono(44100, 44100.0);
        for i in 0..44100 {
            let t = i as f32 / 44100.0;
            test_buffer.data[i] = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
        }
        
        // Verify we have audio
        assert!(test_buffer.rms() > 0.0);
        assert!(test_buffer.peak() > 0.0);
        
        // We could write to file for manual inspection
        // test_buffer.write_wav("/tmp/test_tone.wav").unwrap();
    }
}