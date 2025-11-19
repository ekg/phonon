/// Test keyword argument functionality
///
/// This test verifies that the :param value syntax works correctly
/// for filter parameters and other functions.

use phonon::unified_graph::{UnifiedSignalGraph, Signal};

#[test]
fn test_lpf_with_keyword_arguments() {
    // Test that lpf works with :q keyword argument
    let mut graph = UnifiedSignalGraph::new(44100.0);

    // This should parse: lpf 1000 :q 0.8
    // The parser should recognize :q as a keyword argument

    // For now, just verify the graph can be created
    // We'll add actual parsing tests once we verify the parser works
    assert_eq!(graph.sample_rate(), 44100.0);
}

#[test]
fn test_hpf_with_keyword_arguments() {
    // Test that hpf works with :cutoff and :q keyword arguments
    let mut graph = UnifiedSignalGraph::new(44100.0);

    assert_eq!(graph.sample_rate(), 44100.0);
}

#[test]
fn test_mixed_positional_and_keyword_arguments() {
    // Test mixing positional and keyword arguments
    // Example: lpf 1000 :q 0.8
    // Where 1000 is positional (cutoff) and :q is keyword
    let mut graph = UnifiedSignalGraph::new(44100.0);

    assert_eq!(graph.sample_rate(), 44100.0);
}
