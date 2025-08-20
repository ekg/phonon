//! Modulation routing system for complex signal routing
//! 
//! Enables routing a single modulation source to multiple destinations
//! with individual scaling factors

use crate::signal_graph::{SignalGraph, NodeId, BusId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A modulation route from source to destination
#[derive(Debug, Clone)]
pub struct ModulationRoute {
    pub source: ModulationSource,
    pub destinations: Vec<ModulationDestination>,
}

/// Source of modulation signal
#[derive(Debug, Clone)]
pub enum ModulationSource {
    Bus(BusId),
    Node(NodeId),
    Pattern(String),
    AudioFeature(String),
}

/// Destination for modulation with amount
#[derive(Debug, Clone)]
pub struct ModulationDestination {
    pub target: ModulationTarget,
    pub amount: f32,
    pub mode: ModulationMode,
}

/// Target parameter to modulate
#[derive(Debug, Clone)]
pub enum ModulationTarget {
    NodeParameter {
        node_id: NodeId,
        param: String,
    },
    BusValue(BusId),
    PatternParameter {
        pattern: String,
        param: String,
    },
}

/// How modulation is applied
#[derive(Debug, Clone)]
pub enum ModulationMode {
    Add,        // Add modulation to base value
    Multiply,   // Multiply base value by modulation
    Replace,    // Replace value with modulation
    Bipolar,    // Bipolar modulation (-1 to 1)
}

/// Main modulation router
pub struct ModulationRouter {
    routes: Vec<ModulationRoute>,
    signal_graph: Arc<RwLock<SignalGraph>>,
    
    /// Cached modulation values
    modulation_cache: HashMap<String, f32>,
    
    /// Base parameter values before modulation
    base_values: HashMap<String, f32>,
}

impl ModulationRouter {
    pub fn new(signal_graph: Arc<RwLock<SignalGraph>>) -> Self {
        Self {
            routes: Vec::new(),
            signal_graph,
            modulation_cache: HashMap::new(),
            base_values: HashMap::new(),
        }
    }
    
    /// Add a modulation route
    pub fn add_route(&mut self, route: ModulationRoute) {
        // Store base values for targets
        for dest in &route.destinations {
            let key = self.get_target_key(&dest.target);
            if !self.base_values.contains_key(&key) {
                if let Some(value) = self.get_target_value(&dest.target) {
                    self.base_values.insert(key, value);
                }
            }
        }
        
        self.routes.push(route);
    }
    
    /// Process all modulation routes
    pub fn process(&mut self) {
        // Collect all modulations first to avoid borrow issues
        let mut modulations = Vec::new();
        
        for route in &self.routes {
            let source_value = self.get_source_value(&route.source);
            
            for dest in &route.destinations {
                let target_key = self.get_target_key(&dest.target);
                let base_value = self.base_values.get(&target_key).copied().unwrap_or(0.0);
                
                let modulated_value = match dest.mode {
                    ModulationMode::Add => base_value + source_value * dest.amount,
                    ModulationMode::Multiply => base_value * (1.0 + source_value * dest.amount),
                    ModulationMode::Replace => source_value * dest.amount,
                    ModulationMode::Bipolar => {
                        let bipolar = source_value * 2.0 - 1.0;  // Convert 0-1 to -1 to 1
                        base_value + bipolar * dest.amount
                    }
                };
                
                modulations.push((dest.target.clone(), modulated_value));
            }
        }
        
        // Apply all modulations
        for (target, value) in modulations {
            self.apply_modulation(&target, value);
        }
    }
    
    /// Get value from modulation source
    fn get_source_value(&self, source: &ModulationSource) -> f32 {
        match source {
            ModulationSource::Bus(bus_id) => {
                if let Ok(graph) = self.signal_graph.read() {
                    graph.get_bus_value(bus_id).unwrap_or(0.0)
                } else {
                    0.0
                }
            }
            ModulationSource::Node(_node_id) => {
                // TODO: Get node output value
                0.0
            }
            ModulationSource::Pattern(pattern) => {
                self.modulation_cache.get(pattern).copied().unwrap_or(0.0)
            }
            ModulationSource::AudioFeature(feature) => {
                self.modulation_cache.get(feature).copied().unwrap_or(0.0)
            }
        }
    }
    
    /// Get current value of modulation target
    fn get_target_value(&self, target: &ModulationTarget) -> Option<f32> {
        match target {
            ModulationTarget::BusValue(bus_id) => {
                if let Ok(graph) = self.signal_graph.read() {
                    graph.get_bus_value(bus_id)
                } else {
                    None
                }
            }
            _ => None  // TODO: Implement for other targets
        }
    }
    
    /// Apply modulation to target
    fn apply_modulation(&mut self, target: &ModulationTarget, value: f32) {
        match target {
            ModulationTarget::BusValue(bus_id) => {
                if let Ok(mut graph) = self.signal_graph.write() {
                    graph.set_bus_value(bus_id, value);
                }
            }
            ModulationTarget::NodeParameter { .. } => {
                // TODO: Apply to node parameter
            }
            ModulationTarget::PatternParameter { .. } => {
                // TODO: Apply to pattern parameter
            }
        }
    }
    
    /// Generate unique key for target
    fn get_target_key(&self, target: &ModulationTarget) -> String {
        match target {
            ModulationTarget::NodeParameter { node_id, param } => {
                format!("{}:{}", node_id.0, param)
            }
            ModulationTarget::BusValue(bus_id) => {
                format!("bus:{}", bus_id.0)
            }
            ModulationTarget::PatternParameter { pattern, param } => {
                format!("{}:{}", pattern, param)
            }
        }
    }
    
    /// Update cached modulation value
    pub fn update_cache(&mut self, key: String, value: f32) {
        self.modulation_cache.insert(key, value);
    }
    
    /// Clear all routes
    pub fn clear_routes(&mut self) {
        self.routes.clear();
        self.base_values.clear();
        self.modulation_cache.clear();
    }
    
    /// Parse route statement from DSL
    pub fn parse_route(&mut self, route_str: &str) -> Result<ModulationRoute, String> {
        // Parse: route ~lfo -> { bass.filter.cutoff: 0.3, lead.delay.feedback: 0.2 }
        // or: route ~bass_transient -> ~hats.gain: -0.5
        
        // Simple parsing for now
        if let Some((source_str, dest_str)) = route_str.split_once("->") {
            let source = self.parse_source(source_str.trim())?;
            let destinations = self.parse_destinations(dest_str.trim())?;
            
            Ok(ModulationRoute {
                source,
                destinations,
            })
        } else {
            Err("Invalid route format".to_string())
        }
    }
    
    fn parse_source(&self, source_str: &str) -> Result<ModulationSource, String> {
        if source_str.starts_with('~') {
            Ok(ModulationSource::Bus(BusId(source_str.to_string())))
        } else if source_str.starts_with('@') {
            Ok(ModulationSource::AudioFeature(source_str[1..].to_string()))
        } else {
            Ok(ModulationSource::Pattern(source_str.to_string()))
        }
    }
    
    fn parse_destinations(&self, dest_str: &str) -> Result<Vec<ModulationDestination>, String> {
        let mut destinations = Vec::new();
        
        // Parse single destination: ~hats.gain: -0.5
        if dest_str.contains(':') && !dest_str.contains('{') {
            if let Some((target_str, amount_str)) = dest_str.split_once(':') {
                let amount: f32 = amount_str.trim().parse()
                    .map_err(|_| "Invalid modulation amount")?;
                
                let target = if target_str.starts_with('~') {
                    ModulationTarget::BusValue(BusId(target_str.to_string()))
                } else {
                    // Parse node.param format
                    if let Some((node, param)) = target_str.split_once('.') {
                        ModulationTarget::NodeParameter {
                            node_id: NodeId(node.to_string()),
                            param: param.to_string(),
                        }
                    } else {
                        return Err("Invalid target format".to_string());
                    }
                };
                
                destinations.push(ModulationDestination {
                    target,
                    amount,
                    mode: ModulationMode::Add,
                });
            }
        }
        // TODO: Parse multiple destinations in braces
        
        Ok(destinations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_modulation_router() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let router = ModulationRouter::new(graph);
        assert_eq!(router.routes.len(), 0);
    }
    
    #[test]
    fn test_add_route() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut router = ModulationRouter::new(graph.clone());
        
        // Add a bus to the graph
        {
            let mut g = graph.write().unwrap();
            g.add_bus("~lfo".to_string(), 0.5);
            g.add_bus("~filter_cutoff".to_string(), 1000.0);
        }
        
        let route = ModulationRoute {
            source: ModulationSource::Bus(BusId("~lfo".to_string())),
            destinations: vec![
                ModulationDestination {
                    target: ModulationTarget::BusValue(BusId("~filter_cutoff".to_string())),
                    amount: 500.0,
                    mode: ModulationMode::Add,
                }
            ],
        };
        
        router.add_route(route);
        assert_eq!(router.routes.len(), 1);
    }
    
    #[test]
    fn test_modulation_modes() {
        let base = 100.0;
        let modulation = 0.5;
        let amount = 50.0;
        
        // Add mode
        let add_result = base + modulation * amount;
        assert_eq!(add_result, 125.0);
        
        // Multiply mode
        let mult_result = base * (1.0 + modulation * amount);
        assert_eq!(mult_result, 2600.0);
        
        // Replace mode
        let replace_result = modulation * amount;
        assert_eq!(replace_result, 25.0);
        
        // Bipolar mode
        let bipolar = modulation * 2.0 - 1.0;  // 0.5 -> 0.0
        let bipolar_result = base + bipolar * amount;
        assert_eq!(bipolar_result, 100.0);
    }
    
    #[test]
    fn test_parse_route() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut router = ModulationRouter::new(graph);
        
        let route_str = "~lfo -> ~filter.cutoff: 0.5";
        let route = router.parse_route(route_str);
        
        assert!(route.is_ok());
        let route = route.unwrap();
        assert!(matches!(route.source, ModulationSource::Bus(_)));
        assert_eq!(route.destinations.len(), 1);
        assert_eq!(route.destinations[0].amount, 0.5);
    }
    
    #[test]
    fn test_process_modulation() {
        let graph = Arc::new(RwLock::new(SignalGraph::new(44100.0)));
        let mut router = ModulationRouter::new(graph.clone());
        
        // Setup buses
        {
            let mut g = graph.write().unwrap();
            g.add_bus("~lfo".to_string(), 0.8);
            g.add_bus("~cutoff".to_string(), 1000.0);
        }
        
        // Add modulation route
        let route = ModulationRoute {
            source: ModulationSource::Bus(BusId("~lfo".to_string())),
            destinations: vec![
                ModulationDestination {
                    target: ModulationTarget::BusValue(BusId("~cutoff".to_string())),
                    amount: 500.0,
                    mode: ModulationMode::Add,
                }
            ],
        };
        
        router.add_route(route);
        router.process();
        
        // Check modulated value
        {
            let g = graph.read().unwrap();
            let cutoff = g.get_bus_value(&BusId("~cutoff".to_string())).unwrap();
            assert_eq!(cutoff, 1400.0);  // 1000 + (0.8 * 500)
        }
    }
}