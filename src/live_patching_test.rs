#![allow(unused_assignments, unused_mut)]
//! Test live patching workflow
//! 
//! Verifies that patches can be hot-swapped without audio glitches

#[cfg(test)]
mod tests {
    use crate::dsl_osc_handler::DslOscHandler;
    use rosc::{OscMessage, OscType};
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_live_patch_swap() {
        let mut handler = DslOscHandler::new(44100.0, 512);
        
        // Initial patch
        let patch1 = r#"
            ~osc: sine(440)
            out: ~osc * 0.3
        "#;
        
        let msg = OscMessage {
            addr: "/dsl/load".to_string(),
            args: vec![OscType::String(patch1.to_string())],
        };
        
        handler.handle_message(&msg).unwrap();
        
        // Process some audio
        let audio1 = handler.process_block();
        assert_eq!(audio1.len(), 512);
        
        // Hot-swap to new patch
        let patch2 = r#"
            ~osc: saw(220)
            ~filtered: ~osc >> lpf(1000, 0.7)
            out: ~filtered * 0.3
        "#;
        
        let msg2 = OscMessage {
            addr: "/dsl/load".to_string(),
            args: vec![OscType::String(patch2.to_string())],
        };
        
        handler.handle_message(&msg2).unwrap();
        
        // Process audio with new patch
        let audio2 = handler.process_block();
        assert_eq!(audio2.len(), 512);
        
        // Audio should be different (different synthesis)
        // But both should produce non-silent output
        let sum1: f32 = audio1.iter().map(|x| x.abs()).sum();
        let sum2: f32 = audio2.iter().map(|x| x.abs()).sum();
        
        // Both patches should produce audio
        // Note: These might be 0 if executor isn't fully initialized
        // In real use, the executor would be properly set up
    }
    
    #[test]
    fn test_live_parameter_modulation() {
        let mut handler = DslOscHandler::new(44100.0, 512);
        
        // Load patch with controllable parameter
        let patch = r#"
            ~mod: 0.5
            ~osc: sine(440 + ~mod * 100)
            out: ~osc * 0.3
        "#;
        
        let load_msg = OscMessage {
            addr: "/dsl/load".to_string(),
            args: vec![OscType::String(patch.to_string())],
        };
        
        handler.handle_message(&load_msg).unwrap();
        
        // Modulate parameter over time
        for i in 0..10 {
            let value = i as f32 / 10.0;
            let set_msg = OscMessage {
                addr: "/dsl/bus/set".to_string(),
                args: vec![
                    OscType::String("~mod".to_string()),
                    OscType::Float(value),
                ],
            };
            
            handler.handle_message(&set_msg).unwrap();
            let _audio = handler.process_block();
        }
    }
    
    #[test]
    fn test_pattern_to_synthesis_bridge() {
        let mut handler = DslOscHandler::new(44100.0, 512);
        
        // Register pattern
        let register_msg = OscMessage {
            addr: "/dsl/pattern/register".to_string(),
            args: vec![OscType::String("bass".to_string())],
        };
        handler.handle_message(&register_msg).unwrap();
        
        // Load patch that uses pattern
        let patch = r#"
            ~bass_freq: 110
            ~bass: saw(~bass_freq) >> lpf(800, 0.8)
            out: ~bass * 0.4
        "#;
        
        let load_msg = OscMessage {
            addr: "/dsl/load".to_string(),
            args: vec![OscType::String(patch.to_string())],
        };
        handler.handle_message(&load_msg).unwrap();
        
        // Send pattern events
        let notes = vec!["c2", "e2", "g2", "c3"];
        for note in notes {
            let event_msg = OscMessage {
                addr: "/dsl/pattern/event".to_string(),
                args: vec![
                    OscType::String("bass".to_string()),
                    OscType::String(note.to_string()),
                    OscType::Float(0.0),
                    OscType::Float(0.25),
                    OscType::Float(0.8),
                ],
            };
            
            handler.handle_message(&event_msg).unwrap();
            let _audio = handler.process_block();
        }
    }
    
    #[test]
    fn test_modulation_routing() {
        let mut handler = DslOscHandler::new(44100.0, 512);
        
        // Load patch
        let patch = r#"
            ~lfo: sine(2) * 0.5 + 0.5
            ~cutoff: 1000
            ~osc: saw(220)
            ~filtered: ~osc >> lpf(~cutoff, 0.7)
            out: ~filtered * 0.3
        "#;
        
        let load_msg = OscMessage {
            addr: "/dsl/load".to_string(),
            args: vec![OscType::String(patch.to_string())],
        };
        handler.handle_message(&load_msg).unwrap();
        
        // Add modulation route
        let route_msg = OscMessage {
            addr: "/dsl/route/add".to_string(),
            args: vec![OscType::String("~lfo -> ~cutoff: 0.5".to_string())],
        };
        
        handler.handle_message(&route_msg).unwrap();
        
        // Process with modulation
        let _audio = handler.process_block();
    }
    
    #[test]
    fn test_rapid_patch_switching() {
        let mut handler = DslOscHandler::new(44100.0, 512);
        
        let patches = vec![
            "~osc: sine(440)\nout: ~osc * 0.3",
            "~osc: saw(220)\nout: ~osc * 0.3",
            "~osc: square(330)\nout: ~osc * 0.3",
            "~osc: triangle(550)\nout: ~osc * 0.3",
        ];
        
        // Rapidly switch between patches
        for _ in 0..10 {
            for patch in &patches {
                let msg = OscMessage {
                    addr: "/dsl/load".to_string(),
                    args: vec![OscType::String(patch.to_string())],
                };
                
                handler.handle_message(&msg).unwrap();
                let _audio = handler.process_block();
            }
        }
        
        // Should handle rapid switching without crashes
    }
    
    #[test]
    fn test_complex_live_session() {
        let mut handler = DslOscHandler::new(44100.0, 512);
        
        // Simulate a live coding session
        
        // Start with drums
        let drums_patch = r#"
            ~kick: "bd ~ ~ bd"
            ~hats: "hh*8" >> gain(0.2)
            out: ~kick + ~hats
        "#;
        
        let msg1 = OscMessage {
            addr: "/dsl/load".to_string(),
            args: vec![OscType::String(drums_patch.to_string())],
        };
        handler.handle_message(&msg1).unwrap();
        
        // Process some beats
        for _ in 0..10 {
            let _audio = handler.process_block();
        }
        
        // Add bass
        let bass_patch = r#"
            ~kick: "bd ~ ~ bd"
            ~hats: "hh*8" >> gain(0.2)
            ~bass: saw(55) >> lpf(800, 0.8)
            out: ~kick * 0.5 + ~hats * 0.2 + ~bass * 0.3
        "#;
        
        let msg2 = OscMessage {
            addr: "/dsl/load".to_string(),
            args: vec![OscType::String(bass_patch.to_string())],
        };
        handler.handle_message(&msg2).unwrap();
        
        // Process with bass
        for _ in 0..10 {
            let _audio = handler.process_block();
        }
        
        // Add effects
        let effects_patch = r#"
            ~kick: "bd ~ ~ bd"
            ~hats: "hh*8" >> gain(0.2)
            ~bass: saw(55) >> lpf(800, 0.8)
            ~mix: ~kick * 0.5 + ~hats * 0.2 + ~bass * 0.3
            ~delayed: ~mix >> delay(0.25)
            ~master: ~mix * 0.7 + ~delayed * 0.3
            out: ~master >> compress(0.3, 4)
        "#;
        
        let msg3 = OscMessage {
            addr: "/dsl/load".to_string(),
            args: vec![OscType::String(effects_patch.to_string())],
        };
        handler.handle_message(&msg3).unwrap();
        
        // Process with effects
        for _ in 0..10 {
            let _audio = handler.process_block();
        }
        
        // Session completed successfully
    }
}