/// Test to diagnose kwargs completion issues
///
/// Verifies that the completion system can correctly detect and suggest
/// parameter names for function kwargs.
use phonon::modal_editor::completion::{
    filter_completions, get_completion_context, CompletionContext,
};

#[test]
fn test_gain_kwarg_completion_detection() {
    // User types: "gain :"
    let line = "gain :";
    let cursor_pos = line.len();

    let context = get_completion_context(line, cursor_pos);

    // Should detect Keyword context with "gain"
    if let CompletionContext::Keyword(func_name) = context {
        assert_eq!(func_name, "gain", "Should detect gain as the function");
    } else {
        panic!("Expected Keyword(\"gain\") context, got {:?}", context);
    }
}

#[test]
fn test_gain_kwarg_completion_with_space_only() {
    // User types: "gain " (space but no colon)
    let line = "gain ";
    let cursor_pos = line.len();

    let context = get_completion_context(line, cursor_pos);

    // Should still detect Keyword context with "gain"
    if let CompletionContext::Keyword(func_name) = context {
        assert_eq!(func_name, "gain", "Should detect gain even without colon");
    } else {
        panic!(
            "Expected Keyword(\"gain\") context for 'gain ', got {:?}",
            context
        );
    }

    // Get completions - should have ":amount" WITH the colon
    let completions = filter_completions("", &context, &[], &[]);

    assert!(
        !completions.is_empty(),
        "Should have completions. Got: {:?}",
        completions
    );

    let amount_found = completions.iter().any(|c| c.text == ":amount");
    assert!(
        amount_found,
        "Should have ':amount' (with colon) in completions. Got: {:?}",
        completions.iter().map(|c| &c.text).collect::<Vec<_>>()
    );
}

#[test]
fn test_gain_kwarg_completion_with_space() {
    // User types: "gain : " (with space after colon)
    let line = "gain : ";
    let cursor_pos = line.len();

    let context = get_completion_context(line, cursor_pos);

    // Should still detect Keyword context with "gain"
    if let CompletionContext::Keyword(func_name) = context {
        assert_eq!(func_name, "gain", "Should detect gain even with space");
    } else {
        panic!(
            "Expected Keyword(\"gain\") context with space, got {:?}",
            context
        );
    }
}

#[test]
fn test_gain_kwarg_completion_results() {
    // User types: "gain :"
    let line = "gain :";
    let cursor_pos = line.len();

    let context = get_completion_context(line, cursor_pos);
    let completions = filter_completions("", &context, &[], &[]);

    // Should have at least one completion (":amount")
    assert!(
        !completions.is_empty(),
        "Should have completions for gain kwargs. Got: {:?}",
        completions
    );

    // Check that ":amount" is in the results (with colon prefix since we haven't typed beyond :)
    let amount_found = completions.iter().any(|c| c.text == ":amount");
    assert!(
        amount_found,
        "Should have ':amount' in completions. Got: {:?}",
        completions.iter().map(|c| &c.text).collect::<Vec<_>>()
    );
}

#[test]
fn test_reverb_kwarg_completion() {
    // User types: "reverb 0.8 0.5 :"
    let line = "reverb 0.8 0.5 :";
    let cursor_pos = line.len();

    let context = get_completion_context(line, cursor_pos);

    if let CompletionContext::Keyword(func_name) = context {
        assert_eq!(func_name, "reverb");
    } else {
        panic!("Expected Keyword(\"reverb\") context, got {:?}", context);
    }

    // Get completions
    let completions = filter_completions("", &context, &[], &[]);

    // Should have ":mix" (the optional param for reverb)
    let mix_found = completions.iter().any(|c| c.text == ":mix");
    assert!(
        mix_found,
        "Should have ':mix' for reverb. Got: {:?}",
        completions.iter().map(|c| &c.text).collect::<Vec<_>>()
    );
}

#[test]
fn test_plate_kwarg_completion() {
    // User types: "plate 0.02 3.0 :"
    let line = "plate 0.02 3.0 :";
    let cursor_pos = line.len();

    let context = get_completion_context(line, cursor_pos);

    if let CompletionContext::Keyword(func_name) = context {
        assert_eq!(func_name, "plate");
    } else {
        panic!("Expected Keyword(\"plate\") context, got {:?}", context);
    }

    // Get completions
    let completions = filter_completions("", &context, &[], &[]);

    // Should have multiple params: diffusion, damping, mod_depth, mix
    let param_names: Vec<&str> = completions.iter().map(|c| c.text.as_str()).collect();

    assert!(
        param_names.contains(&":diffusion"),
        "Should have ':diffusion'. Got: {:?}",
        param_names
    );
    assert!(
        param_names.contains(&":damping"),
        "Should have ':damping'. Got: {:?}",
        param_names
    );
    assert!(
        param_names.contains(&":mix"),
        "Should have ':mix'. Got: {:?}",
        param_names
    );
}

#[test]
fn test_lpf_kwarg_completion() {
    // User types: "lpf 800 :"
    let line = "lpf 800 :";
    let cursor_pos = line.len();

    let context = get_completion_context(line, cursor_pos);

    if let CompletionContext::Keyword(func_name) = context {
        assert_eq!(func_name, "lpf");
    } else {
        panic!("Expected Keyword(\"lpf\") context, got {:?}", context);
    }

    // Get completions
    let completions = filter_completions("", &context, &[], &[]);

    // Should have both ":cutoff" and ":q"
    let param_names: Vec<&str> = completions.iter().map(|c| c.text.as_str()).collect();

    assert!(
        param_names.contains(&":cutoff"),
        "Should have ':cutoff'. Got: {:?}",
        param_names
    );
    assert!(
        param_names.contains(&":q"),
        "Should have ':q'. Got: {:?}",
        param_names
    );
}

#[test]
fn test_kwarg_partial_completion() {
    // User types: "gain :am" (partial parameter name)
    let line = "gain :am";
    let cursor_pos = line.len();

    let context = get_completion_context(line, cursor_pos);

    if let CompletionContext::Keyword(func_name) = context {
        assert_eq!(func_name, "gain");
    } else {
        panic!("Expected Keyword(\"gain\") context, got {:?}", context);
    }

    // Get completions with partial match
    let completions = filter_completions(":am", &context, &[], &[]);

    // Should still have ":amount" (fuzzy match on "am")
    assert!(
        !completions.is_empty(),
        "Should have completions for partial match. Got: {:?}",
        completions
    );

    let amount_found = completions.iter().any(|c| c.text == "amount");
    assert!(
        amount_found,
        "Should fuzzy match ':amount' from ':am'. Got: {:?}",
        completions.iter().map(|c| &c.text).collect::<Vec<_>>()
    );
}

#[test]
fn test_kwarg_in_chain() {
    // User types: "saw 55 # lpf 800 :"
    let line = "saw 55 # lpf 800 :";
    let cursor_pos = line.len();

    let context = get_completion_context(line, cursor_pos);

    if let CompletionContext::Keyword(func_name) = context {
        assert_eq!(func_name, "lpf", "Should detect lpf in chain, not saw");
    } else {
        panic!("Expected Keyword(\"lpf\") in chain, got {:?}", context);
    }
}
