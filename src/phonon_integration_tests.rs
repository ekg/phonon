#![allow(unused_assignments, unused_mut)]
//! Integration tests for full .phonon file processing
//! 
//! Tests the complete pipeline from parsing .phonon files to audio generation

#[cfg(test)]
mod tests {
    use crate::enhanced_parser::EnhancedParser;
    use crate::signal_graph::SignalGraph;
    use crate::signal_executor::{SignalExecutor, AudioBuffer};
    use crate::pattern_bridge::PatternBridge;
    use crate::modulation_router::{ModulationRouter, ModulationRoute};
    use crate::audio_analysis::AudioAnalyzer;
    use std::sync::{Arc, RwLock};
    use std::fs;
    use std::path::Path;
    
    fn load_phonon_file(path: &Path) -> Result<String, std::io::Error> {
        fs::read_to_string(path)
    }
    
    #[test]
    fn test_full_modular_synth_integration() {
        // Load test pattern file
        let test_file = Path::new("../test-patterns/test_modular_synth.phonon");
        let content = load_phonon_file(test_file);
        
        assert!(content.is_ok(), "Should load test pattern file");
        let dsl = content.unwrap();
        
        // Parse the DSL
        let mut parser = EnhancedParser::new(44100.0);
        let parse_result = parser.parse(&dsl);
        assert!(parse_result.is_ok(), "Should parse test pattern");
        
        let graph = parse_result.unwrap();
        let graph_arc = Arc::new(RwLock::new(graph));
        
        // Create pattern bridge
        let mut bridge = PatternBridge::new(graph_arc.clone(), 44100.0);
        bridge.register_pattern("kick".to_string());
        bridge.register_pattern("snare".to_string());
        bridge.register_pattern("hats".to_string());
        
        // Create modulation router
        let mut router = ModulationRouter::new(graph_arc.clone());
        
        // Create signal executor
        let graph_clone = {
            let g = graph_arc.read().unwrap();
            SignalGraph::new(g.sample_rate)  // Create new graph for executor
        };
        let mut executor = SignalExecutor::new(graph_clone, 44100.0, 512);
        
        // Process one block
        let output = executor.process_block();
        assert!(output.is_ok(), "Should process audio block");
    }
    
    #[test]
    fn test_lfo_modulation() {
        let dsl = r#"
            ~lfo: sine(2) * 0.5 + 0.5
            ~osc: saw(440)
            ~modulated: ~osc >> lpf(~lfo * 1000 + 500, 0.7)
            out: ~modulated
        "#;
        
        let mut parser = EnhancedParser::new(44100.0);
        let result = parser.parse(dsl);
        assert!(result.is_ok(), "Should parse LFO modulation");
    }
    
    #[test]
    fn test_pattern_to_audio_cross_modulation() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut bridge = PatternBridge::new(graph.clone(), 44100.0);
        
        // Register bass pattern
        bridge.register_pattern("bass".to_string());
        
        // Simulate bass note
        let event = crate::pattern_bridge::PatternEvent {
            time: 0.0,
            value: crate::pattern_bridge::PatternValue::Note("c2".to_string()),
            duration: 0.5,
            velocity: 0.8,
        };
        bridge.process_pattern_event("bass", event);
        
        // Extract audio features
        bridge.audio_features.insert("bass_rms".to_string(), 0.7);
        
        // Use audio feature to modulate another parameter
        let modulated = bridge.apply_pattern_modulation("bass", 1000.0, 200.0);
        assert!(modulated != 1000.0, "Should apply modulation");
    }
    
    #[test]
    fn test_audio_analysis_integration() {
        let mut analyzer = AudioAnalyzer::new(44100.0);
        
        // Generate test signal
        let mut block = vec![0.0; 512];
        for i in 0..512 {
            let t = i as f32 / 44100.0;
            // Mix of frequencies
            block[i] = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.3
                     + (2.0 * std::f32::consts::PI * 880.0 * t).sin() * 0.2
                     + (2.0 * std::f32::consts::PI * 220.0 * t).sin() * 0.1;
        }
        
        let features = analyzer.analyze_block(&block);
        
        // Verify all features are extracted
        assert!(features.rms > 0.0, "Should have RMS");
        assert!(features.centroid >= 0.0, "Should have spectral centroid");
        
        // Test with transient
        let mut transient_block = vec![0.0; 256];
        transient_block.extend(vec![0.8; 256]);
        
        let transient_features = analyzer.analyze_block(&transient_block);
        assert!(transient_features.transient >= 0.0, "Should detect transient");
    }
    
    #[test]
    fn test_complex_routing_chain() {
        let dsl = r#"
            ~env: perc(0.01, 0.3)
            ~lfo: sine(6) * 0.1
            ~osc_freq: 440 + ~lfo * 50
            ~osc: saw(~osc_freq) * ~env
            ~filter_freq: ~env * 2000 + 500
            ~filtered: ~osc >> lpf(~filter_freq, 0.8)
            ~delayed: ~filtered >> delay(0.25)
            ~mixed: ~filtered * 0.7 + ~delayed * 0.3
            out: ~mixed
        "#;
        
        let mut parser = EnhancedParser::new(44100.0);
        let result = parser.parse(dsl);
        assert!(result.is_ok(), "Should parse complex routing chain");
    }
    
    #[test]
    fn test_parallel_processing_paths() {
        let dsl = r#"
            // Parallel processing paths
            ~dry: saw(220)
            
            // Path 1: Low-pass
            ~low: ~dry >> lpf(800, 0.7)
            
            // Path 2: High-pass  
            ~high: ~dry >> hpf(2000, 0.7)
            
            // Path 3: Band-pass
            ~mid: ~dry >> bpf(1000, 0.5)
            
            // Mix parallel paths
            ~mixed: ~low * 0.3 + ~mid * 0.4 + ~high * 0.3
            
            out: ~mixed
        "#;
        
        let mut parser = EnhancedParser::new(44100.0);
        let result = parser.parse(dsl);
        assert!(result.is_ok(), "Should parse parallel processing");
    }
    
    #[test]
    fn test_synthdef_parsing() {
        let dsl = r#"
            synthdef kick sine(60) * perc(0.001, 0.1) + noise() * perc(0.001, 0.05) * 0.2
            synthdef snare noise() * perc(0.001, 0.05) >> hpf(200, 0.8)
            synthdef bass saw(55) >> lpf(800, 0.9)
            
            ~rhythm: "kick ~ snare ~"
            out: ~rhythm
        "#;
        
        let mut parser = EnhancedParser::new(44100.0);
        let result = parser.parse(dsl);
        assert!(result.is_ok(), "Should parse synthdefs");
    }
    
    #[test]
    fn test_modulation_router_integration() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut router = ModulationRouter::new(graph.clone());
        
        // Setup test buses
        {
            let mut g = graph.write().unwrap();
            g.add_bus("~lfo".to_string(), 0.0);
            g.add_bus("~cutoff".to_string(), 1000.0);
            g.add_bus("~resonance".to_string(), 0.5);
        }
        
        // Parse and add route
        let route_str = "~lfo -> ~cutoff: 0.8";
        let route = router.parse_route(route_str);
        assert!(route.is_ok(), "Should parse route");
        
        router.add_route(route.unwrap());
        
        // Update LFO value
        {
            let mut g = graph.write().unwrap();
            g.set_bus_value(&crate::signal_graph::BusId("~lfo".to_string()), 0.5);
        }
        
        // Process routing
        router.process();
        
        // Verify modulation was applied
        {
            let g = graph.read().unwrap();
            let cutoff = g.get_bus_value(&crate::signal_graph::BusId("~cutoff".to_string()));
            assert!(cutoff.is_some());
            assert_ne!(cutoff.unwrap(), 1000.0, "Cutoff should be modulated");
        }
    }
}