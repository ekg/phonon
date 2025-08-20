//! OSC message handler for the modular synthesis DSL
//! 
//! Handles OSC messages to load, execute, and modify DSL patches

use crate::enhanced_parser::EnhancedParser;
use crate::signal_graph::SignalGraph;
use crate::signal_executor::SignalExecutor;
use crate::pattern_bridge::{PatternBridge, PatternEvent, PatternValue};
use crate::modulation_router::ModulationRouter;
use rosc::{OscMessage, OscType};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use tracing::{info, error, debug};

/// Main DSL handler that manages patches and execution
pub struct DslOscHandler {
    /// Current signal graph
    graph: Arc<RwLock<SignalGraph>>,
    
    /// Pattern bridge for cross-modulation
    pattern_bridge: Arc<RwLock<PatternBridge>>,
    
    /// Modulation router
    modulation_router: Arc<RwLock<ModulationRouter>>,
    
    /// Signal executor
    executor: Arc<RwLock<Option<SignalExecutor>>>,
    
    /// Current DSL patch text
    current_patch: String,
    
    /// Sample rate
    sample_rate: f32,
    
    /// Block size for processing
    block_size: usize,
}

impl DslOscHandler {
    pub fn new(sample_rate: f32, block_size: usize) -> Self {
        let graph = Arc::new(RwLock::new(SignalGraph::new(sample_rate)));
        let pattern_bridge = Arc::new(RwLock::new(
            PatternBridge::new(graph.clone(), sample_rate)
        ));
        let modulation_router = Arc::new(RwLock::new(
            ModulationRouter::new(graph.clone())
        ));
        
        Self {
            graph,
            pattern_bridge,
            modulation_router,
            executor: Arc::new(RwLock::new(None)),
            current_patch: String::new(),
            sample_rate,
            block_size,
        }
    }
    
    /// Handle incoming OSC message
    pub fn handle_message(&mut self, msg: &OscMessage) -> Result<(), String> {
        match msg.addr.as_str() {
            "/dsl/load" => self.handle_load_patch(msg),
            "/dsl/reload" => self.handle_reload_patch(),
            "/dsl/clear" => self.handle_clear_patch(),
            "/dsl/bus/set" => self.handle_set_bus(msg),
            "/dsl/pattern/event" => self.handle_pattern_event(msg),
            "/dsl/pattern/register" => self.handle_register_pattern(msg),
            "/dsl/route/add" => self.handle_add_route(msg),
            "/dsl/analyze" => self.handle_analyze(msg),
            "/dsl/synthdef" => self.handle_synthdef(msg),
            _ => {
                debug!("Unknown DSL OSC address: {}", msg.addr);
                Ok(())
            }
        }
    }
    
    /// Load a new DSL patch
    fn handle_load_patch(&mut self, msg: &OscMessage) -> Result<(), String> {
        if let Some(OscType::String(patch_text)) = msg.args.get(0) {
            info!("Loading DSL patch: {} chars", patch_text.len());
            
            // Parse the DSL
            let mut parser = EnhancedParser::new(self.sample_rate);
            let new_graph = parser.parse(patch_text)?;
            
            // Update the graph
            {
                let mut graph = self.graph.write().unwrap();
                *graph = new_graph;
            }
            
            // Create new executor
            let graph_copy = {
                let g = self.graph.read().unwrap();
                SignalGraph::new(g.sample_rate)  // Create copy for executor
            };
            
            let new_executor = SignalExecutor::new(graph_copy, self.sample_rate, self.block_size);
            
            {
                let mut exec = self.executor.write().unwrap();
                *exec = Some(new_executor);
            }
            
            self.current_patch = patch_text.clone();
            
            info!("DSL patch loaded successfully");
            Ok(())
        } else {
            Err("Missing patch text in /dsl/load message".to_string())
        }
    }
    
    /// Reload the current patch (for hot-swapping)
    fn handle_reload_patch(&mut self) -> Result<(), String> {
        if !self.current_patch.is_empty() {
            let patch = self.current_patch.clone();
            let msg = OscMessage {
                addr: "/dsl/load".to_string(),
                args: vec![OscType::String(patch)],
            };
            self.handle_load_patch(&msg)
        } else {
            Err("No patch to reload".to_string())
        }
    }
    
    /// Clear the current patch
    fn handle_clear_patch(&mut self) -> Result<(), String> {
        {
            let mut graph = self.graph.write().unwrap();
            graph.clear();
        }
        
        {
            let mut exec = self.executor.write().unwrap();
            *exec = None;
        }
        
        {
            let mut router = self.modulation_router.write().unwrap();
            router.clear_routes();
        }
        
        self.current_patch.clear();
        
        info!("DSL patch cleared");
        Ok(())
    }
    
    /// Set a bus value
    fn handle_set_bus(&mut self, msg: &OscMessage) -> Result<(), String> {
        if msg.args.len() >= 2 {
            if let (Some(OscType::String(bus_name)), Some(OscType::Float(value))) = 
                (msg.args.get(0), msg.args.get(1)) {
                
                let bus_id = crate::signal_graph::BusId(bus_name.clone());
                
                {
                    let mut graph = self.graph.write().unwrap();
                    graph.set_bus_value(&bus_id, *value);
                }
                
                debug!("Set bus {} to {}", bus_name, value);
                Ok(())
            } else {
                Err("Invalid arguments for /dsl/bus/set".to_string())
            }
        } else {
            Err("Missing arguments for /dsl/bus/set".to_string())
        }
    }
    
    /// Handle pattern event
    fn handle_pattern_event(&mut self, msg: &OscMessage) -> Result<(), String> {
        if msg.args.len() >= 5 {
            if let (
                Some(OscType::String(pattern_name)),
                Some(OscType::String(value_str)),
                Some(OscType::Float(time)),
                Some(OscType::Float(duration)),
                Some(OscType::Float(velocity)),
            ) = (
                msg.args.get(0),
                msg.args.get(1),
                msg.args.get(2),
                msg.args.get(3),
                msg.args.get(4),
            ) {
                let value = if value_str.chars().all(|c| c.is_numeric() || c == '.') {
                    PatternValue::Number(value_str.parse().unwrap_or(0.0))
                } else if value_str.contains(char::is_numeric) {
                    PatternValue::Note(value_str.clone())
                } else {
                    PatternValue::Sample(value_str.clone())
                };
                
                let event = PatternEvent {
                    time: *time as f64,
                    value,
                    duration: *duration as f64,
                    velocity: *velocity,
                };
                
                {
                    let mut bridge = self.pattern_bridge.write().unwrap();
                    bridge.process_pattern_event(pattern_name, event);
                }
                
                Ok(())
            } else {
                Err("Invalid arguments for /dsl/pattern/event".to_string())
            }
        } else {
            Err("Missing arguments for /dsl/pattern/event".to_string())
        }
    }
    
    /// Register a new pattern
    fn handle_register_pattern(&mut self, msg: &OscMessage) -> Result<(), String> {
        if let Some(OscType::String(pattern_name)) = msg.args.get(0) {
            {
                let mut bridge = self.pattern_bridge.write().unwrap();
                bridge.register_pattern(pattern_name.clone());
            }
            
            info!("Registered pattern: {}", pattern_name);
            Ok(())
        } else {
            Err("Missing pattern name in /dsl/pattern/register".to_string())
        }
    }
    
    /// Add a modulation route
    fn handle_add_route(&mut self, msg: &OscMessage) -> Result<(), String> {
        if let Some(OscType::String(route_str)) = msg.args.get(0) {
            {
                let mut router = self.modulation_router.write().unwrap();
                let route = router.parse_route(route_str)?;
                router.add_route(route);
            }
            
            debug!("Added modulation route: {}", route_str);
            Ok(())
        } else {
            Err("Missing route string in /dsl/route/add".to_string())
        }
    }
    
    /// Handle audio analysis request
    fn handle_analyze(&mut self, msg: &OscMessage) -> Result<(), String> {
        // This would trigger analysis and send results back via OSC
        // For now, just acknowledge
        debug!("Analysis requested");
        Ok(())
    }
    
    /// Handle synthdef definition
    fn handle_synthdef(&mut self, msg: &OscMessage) -> Result<(), String> {
        if msg.args.len() >= 2 {
            if let (Some(OscType::String(name)), Some(OscType::String(definition))) = 
                (msg.args.get(0), msg.args.get(1)) {
                
                // Store synthdef for later use
                // This would be integrated with the parser
                info!("Defined synthdef: {}", name);
                Ok(())
            } else {
                Err("Invalid arguments for /dsl/synthdef".to_string())
            }
        } else {
            Err("Missing arguments for /dsl/synthdef".to_string())
        }
    }
    
    /// Process audio block (called from audio thread)
    pub fn process_block(&mut self) -> Vec<f32> {
        // Process modulation routing
        {
            let mut router = self.modulation_router.write().unwrap();
            router.process();
        }
        
        // Process audio
        if let Ok(mut exec) = self.executor.write() {
            if let Some(ref mut executor) = *exec {
                if let Ok(buffer) = executor.process_block() {
                    return buffer.data;
                }
            }
        }
        
        // Return silence if no executor
        vec![0.0; self.block_size]
    }
    
    /// Get current patch text
    pub fn get_patch(&self) -> &str {
        &self.current_patch
    }
    
    /// Get bus value
    pub fn get_bus_value(&self, bus_name: &str) -> Option<f32> {
        let bus_id = crate::signal_graph::BusId(bus_name.to_string());
        if let Ok(graph) = self.graph.read() {
            graph.get_bus_value(&bus_id)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_handler() {
        let handler = DslOscHandler::new(44100.0, 512);
        assert_eq!(handler.get_patch(), "");
    }
    
    #[test]
    fn test_load_patch() {
        let mut handler = DslOscHandler::new(44100.0, 512);
        
        let patch = "~lfo: sine(2)\nout: ~lfo";
        let msg = OscMessage {
            addr: "/dsl/load".to_string(),
            args: vec![OscType::String(patch.to_string())],
        };
        
        let result = handler.handle_message(&msg);
        assert!(result.is_ok());
        assert_eq!(handler.get_patch(), patch);
    }
    
    #[test]
    fn test_set_bus() {
        let mut handler = DslOscHandler::new(44100.0, 512);
        
        // First load a patch with a bus
        let patch = "~test: 0.0\nout: ~test";
        let load_msg = OscMessage {
            addr: "/dsl/load".to_string(),
            args: vec![OscType::String(patch.to_string())],
        };
        handler.handle_message(&load_msg).unwrap();
        
        // Set bus value
        let msg = OscMessage {
            addr: "/dsl/bus/set".to_string(),
            args: vec![
                OscType::String("~test".to_string()),
                OscType::Float(0.5),
            ],
        };
        
        let result = handler.handle_message(&msg);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_register_pattern() {
        let mut handler = DslOscHandler::new(44100.0, 512);
        
        let msg = OscMessage {
            addr: "/dsl/pattern/register".to_string(),
            args: vec![OscType::String("kick".to_string())],
        };
        
        let result = handler.handle_message(&msg);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_pattern_event() {
        let mut handler = DslOscHandler::new(44100.0, 512);
        
        // First register the pattern
        let register_msg = OscMessage {
            addr: "/dsl/pattern/register".to_string(),
            args: vec![OscType::String("bass".to_string())],
        };
        handler.handle_message(&register_msg).unwrap();
        
        // Send pattern event
        let msg = OscMessage {
            addr: "/dsl/pattern/event".to_string(),
            args: vec![
                OscType::String("bass".to_string()),
                OscType::String("c3".to_string()),
                OscType::Float(0.0),
                OscType::Float(0.5),
                OscType::Float(0.8),
            ],
        };
        
        let result = handler.handle_message(&msg);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_clear_patch() {
        let mut handler = DslOscHandler::new(44100.0, 512);
        
        // Load a patch
        let patch = "~test: sine(440)\nout: ~test";
        let load_msg = OscMessage {
            addr: "/dsl/load".to_string(),
            args: vec![OscType::String(patch.to_string())],
        };
        handler.handle_message(&load_msg).unwrap();
        
        // Clear it
        let msg = OscMessage {
            addr: "/dsl/clear".to_string(),
            args: vec![],
        };
        
        let result = handler.handle_message(&msg);
        assert!(result.is_ok());
        assert_eq!(handler.get_patch(), "");
    }
}