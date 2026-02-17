# Pattern Validation Framework Architecture

**Status**: Design Document
**Date**: 2026-01-29
**Author**: Design Agent

---

## Executive Summary

This document describes the architecture for a comprehensive pattern validation framework in Phonon. The framework addresses critical gaps in current error handling:

1. **Mini-notation parser returns silence on error** - users get no feedback
2. **Silent fallbacks throughout** - `unwrap_or(1.0)` hides bugs
3. **No type checking** - Pattern<String> can be used where Pattern<f64> expected
4. **No parameter range validation** - frequencies, gains out of range
5. **No runtime NaN/Infinity detection** - propagates through signal chain

The framework provides four validation layers: Parse-time, Type-checking, Semantic, and Runtime.

---

## Architectural Principles

### 1. Fail Fast, Fail Loud

**Current behavior** (bad):
```rust
// mini_notation_v3.rs - silently returns empty pattern
pub fn parse_mini_notation(input: &str) -> Pattern<String> {
    // Returns Pattern::silence() on parse failure - user sees nothing
}

// compositional_compiler.rs - silent fallback
.fmap(|s| s.parse::<f64>().unwrap_or(1.0))  // Hides parsing errors
```

**New behavior** (good):
```rust
pub fn parse_mini_notation(input: &str) -> Result<Pattern<String>, ValidationError> {
    // Returns detailed error with line/column, suggestion
}
```

### 2. Errors Are Data

Validation errors are structured data that can be:
- Displayed to users (with musical context)
- Logged for debugging
- Aggregated for metrics
- Used for IDE integration (red squiggles)

### 3. Graceful Degradation at Boundaries

- **Parse errors**: Stop compilation, show error
- **Type errors**: Stop compilation, show error
- **Semantic warnings**: Compile but warn
- **Runtime errors**: Log, use safe fallback, continue playing

### 4. Musical Error Messages

Errors should speak the user's language:
```
❌ Parse Error at line 3:15

  ~drums: s "bd sn hh(3,8"
                       ^

Error: Unclosed Euclidean pattern - missing ')'
💡 Hint: Try "bd sn hh(3,8)" - Euclidean rhythms need closing parenthesis
```

---

## Error Type Hierarchy

```rust
/// Top-level validation error enum
pub enum ValidationError {
    /// Parse-time errors (syntax)
    Parse(ParseError),
    /// Type errors (mismatched pattern types)
    Type(TypeError),
    /// Semantic errors (invalid values, impossible patterns)
    Semantic(SemanticError),
    /// Runtime errors (NaN, overflow, etc.)
    Runtime(RuntimeError),
}

/// Parse-time errors with location info
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub location: SourceLocation,
    pub source_line: String,
    pub hint: Option<String>,
}

pub enum ParseErrorKind {
    // Mini-notation specific
    UnexpectedToken { found: String, expected: Vec<String> },
    UnmatchedBracket { opener: char, line: usize },
    InvalidEuclidean { reason: String },
    EmptyAlternation,
    InvalidReplication { value: String },
    InvalidSlowDown { value: String },

    // DSL-level
    UnknownTransform { name: String, did_you_mean: Option<String> },
    WrongArity { name: String, expected: usize, got: usize },
    UnknownBus { name: String },
    UnknownSample { name: String, bank: String },
    InvalidBpm { value: String },

    // General
    UnexpectedEof,
    InvalidCharacter { char: char },
}

/// Type errors for pattern operations
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub location: Option<SourceLocation>,
    pub expected: String,
    pub got: String,
}

pub enum TypeErrorKind {
    PatternTypeMismatch,      // Pattern<String> where Pattern<f64> expected
    ArithmeticOnStrings,      // "bd sn" + "hh"
    InvalidCoercion,          // Can't convert "foo" to f64
    IncompatiblePatterns,     // Different step structures
}

/// Semantic errors (valid syntax, invalid meaning)
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub location: Option<SourceLocation>,
    pub severity: Severity,
}

pub enum SemanticErrorKind {
    // Parameter ranges
    FrequencyOutOfRange { freq: f64, min: f64, max: f64 },
    GainOutOfRange { gain: f64, min: f64, max: f64 },
    ResonanceOutOfRange { q: f64, min: f64, max: f64 },
    TimeNegative { param: String, value: f64 },

    // Pattern structure
    EmptyPattern,
    InfiniteLoop { description: String },
    ExcessiveNesting { depth: usize, max: usize },

    // Composition
    SelfReference { bus: String },
    CyclicDependency { chain: Vec<String> },

    // Musical
    BpmTooFast { bpm: f64 },
    BpmTooSlow { bpm: f64 },
}

pub enum Severity {
    Error,    // Stop compilation
    Warning,  // Compile but warn
    Info,     // Informational
}

/// Runtime errors (detected during evaluation)
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub context: String,
    pub recovery: RecoveryAction,
}

pub enum RuntimeErrorKind {
    NaN { operation: String },
    Infinity { operation: String },
    DivisionByZero,
    Overflow { value: String },
    Underflow { value: String },
    SampleNotFound { name: String },
    BufferOverrun { requested: usize, available: usize },
}

pub enum RecoveryAction {
    UseDefault(f64),
    UsePrevious,
    Silence,
    Skip,
}

/// Source location for error reporting
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub line: usize,      // 1-indexed
    pub column: usize,    // 1-indexed
    pub span: (usize, usize),  // byte offset (start, end)
    pub file: Option<String>,
}
```

---

## Layer 1: Parse-Time Validation

### Mini-Notation Parser Changes

**Current signature**:
```rust
pub fn parse_mini_notation(input: &str) -> Pattern<String>
```

**New signature**:
```rust
pub fn parse_mini_notation(input: &str) -> Result<Pattern<String>, ParseError>
```

### Implementation Strategy

```rust
// mini_notation_v3.rs

pub fn parse_mini_notation(input: &str) -> Result<Pattern<String>, ParseError> {
    let input = input.trim();

    // Quick validation
    validate_brackets(input)?;
    validate_characters(input)?;

    // Tokenize with error tracking
    let tokens = tokenize_with_errors(input)?;

    // Parse with detailed errors
    let ast = parse_tokens_to_ast(&tokens, input)?;

    // Convert to pattern
    ast_to_pattern(ast)
}

fn validate_brackets(input: &str) -> Result<(), ParseError> {
    let mut stack: Vec<(char, usize, usize)> = Vec::new();

    for (idx, ch) in input.char_indices() {
        match ch {
            '[' | '<' | '(' => {
                let line = input[..idx].matches('\n').count() + 1;
                let col = idx - input[..idx].rfind('\n').map(|i| i + 1).unwrap_or(0) + 1;
                stack.push((ch, line, col));
            }
            ']' => {
                if let Some((opener, _, _)) = stack.pop() {
                    if opener != '[' {
                        return Err(ParseError {
                            kind: ParseErrorKind::UnmatchedBracket { opener, line: 1 },
                            location: SourceLocation::from_offset(input, idx),
                            source_line: get_line(input, idx),
                            hint: Some(format!("Expected ']' to match '[', found mismatched '{}'", opener)),
                        });
                    }
                } else {
                    return Err(ParseError {
                        kind: ParseErrorKind::UnmatchedBracket { opener: ']', line: 1 },
                        location: SourceLocation::from_offset(input, idx),
                        source_line: get_line(input, idx),
                        hint: Some("Unexpected ']' - no matching '[' found".into()),
                    });
                }
            }
            '>' => {
                if let Some((opener, _, _)) = stack.pop() {
                    if opener != '<' {
                        return Err(ParseError {
                            kind: ParseErrorKind::UnmatchedBracket { opener, line: 1 },
                            location: SourceLocation::from_offset(input, idx),
                            source_line: get_line(input, idx),
                            hint: Some(format!("Expected '>' to match '<', found mismatched '{}'", opener)),
                        });
                    }
                }
            }
            ')' => {
                if let Some((opener, line, col)) = stack.pop() {
                    if opener != '(' {
                        return Err(ParseError {
                            kind: ParseErrorKind::InvalidEuclidean {
                                reason: "Mismatched parentheses in Euclidean pattern".into()
                            },
                            location: SourceLocation::from_offset(input, idx),
                            source_line: get_line(input, idx),
                            hint: Some(format!("Opening '(' at line {}:{} doesn't match ')'", line, col)),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    if let Some((opener, line, col)) = stack.pop() {
        let hint = match opener {
            '[' => "Fast sequence '[...]' needs closing ']'",
            '<' => "Alternation '<...>' needs closing '>'",
            '(' => "Euclidean pattern '(k,n)' needs closing ')'",
            _ => "Bracket needs to be closed",
        };
        return Err(ParseError {
            kind: ParseErrorKind::UnmatchedBracket { opener, line },
            location: SourceLocation { line, column: col, span: (0, 0), file: None },
            source_line: get_line_by_number(input, line),
            hint: Some(hint.into()),
        });
    }

    Ok(())
}
```

### Euclidean Pattern Validation

```rust
fn validate_euclidean(pulses: &str, steps: &str, rotation: Option<&str>) -> Result<(i32, i32, i32), ParseError> {
    let pulses: i32 = pulses.parse().map_err(|_| ParseError {
        kind: ParseErrorKind::InvalidEuclidean {
            reason: format!("Pulses '{}' is not a valid integer", pulses)
        },
        // ... location info
    })?;

    let steps: i32 = steps.parse().map_err(|_| ParseError {
        kind: ParseErrorKind::InvalidEuclidean {
            reason: format!("Steps '{}' is not a valid integer", steps)
        },
        // ...
    })?;

    // Semantic validation
    if steps <= 0 {
        return Err(ParseError {
            kind: ParseErrorKind::InvalidEuclidean {
                reason: format!("Steps must be positive, got {}", steps)
            },
            hint: Some("Euclidean patterns need at least 1 step: e.g., bd(3,8)".into()),
            // ...
        });
    }

    if pulses < 0 {
        return Err(ParseError {
            kind: ParseErrorKind::InvalidEuclidean {
                reason: format!("Pulses cannot be negative, got {}", pulses)
            },
            hint: Some("Use 0 pulses for silence, positive for hits: e.g., bd(3,8)".into()),
            // ...
        });
    }

    if pulses > steps {
        return Err(ParseError {
            kind: ParseErrorKind::InvalidEuclidean {
                reason: format!("More pulses ({}) than steps ({})", pulses, steps)
            },
            hint: Some("Pulses cannot exceed steps: e.g., bd(3,8) not bd(10,8)".into()),
            // ...
        });
    }

    let rotation = rotation.unwrap_or("0").parse().unwrap_or(0);

    Ok((pulses, steps, rotation))
}
```

### Integration with Existing DiagnosticError

Leverage the existing `error_diagnostics.rs` module:

```rust
// Extend error_diagnostics.rs

impl From<ParseError> for DiagnosticError {
    fn from(err: ParseError) -> Self {
        DiagnosticError {
            line: err.location.line,
            column: err.location.column,
            message: err.kind.to_string(),
            hint: err.hint,
            source_line: Some(err.source_line),
        }
    }
}

impl ParseErrorKind {
    pub fn to_string(&self) -> String {
        match self {
            Self::UnexpectedToken { found, expected } => {
                format!("Unexpected '{}', expected one of: {}", found, expected.join(", "))
            }
            Self::UnmatchedBracket { opener, .. } => {
                format!("Unmatched bracket '{}'", opener)
            }
            Self::InvalidEuclidean { reason } => {
                format!("Invalid Euclidean pattern: {}", reason)
            }
            Self::EmptyAlternation => {
                "Empty alternation '<>' not allowed".into()
            }
            // ... etc
        }
    }
}
```

---

## Layer 2: Type Checking

### Pattern Type System

Phonon patterns are generic: `Pattern<T>` where `T: Clone + Send + Sync`. However, the DSL currently doesn't track types, leading to runtime issues.

### Type Inference Strategy

```rust
/// Inferred type for a pattern expression
#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    /// Numeric pattern (frequencies, gains, etc.)
    Numeric,
    /// String pattern (sample names, notes)
    String,
    /// Boolean pattern (for control flow)
    Bool,
    /// Unknown (needs inference)
    Unknown,
    /// Error type (propagates through expressions)
    Error,
}

/// Type-annotated expression
pub struct TypedExpr {
    pub expr: Expr,
    pub ty: PatternType,
    pub span: SourceLocation,
}

/// Type checker for DSL expressions
pub struct TypeChecker {
    buses: HashMap<String, PatternType>,
    errors: Vec<TypeError>,
}

impl TypeChecker {
    pub fn check_program(&mut self, statements: &[Statement]) -> Result<Vec<TypedStatement>, Vec<TypeError>> {
        let mut typed = Vec::new();

        for stmt in statements {
            match self.check_statement(stmt) {
                Ok(ts) => typed.push(ts),
                Err(e) => self.errors.push(e),
            }
        }

        if self.errors.is_empty() {
            Ok(typed)
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    fn check_statement(&mut self, stmt: &Statement) -> Result<TypedStatement, TypeError> {
        match stmt {
            Statement::BusAssign { name, expr } => {
                let typed_expr = self.check_expr(expr)?;
                self.buses.insert(name.clone(), typed_expr.ty.clone());
                Ok(TypedStatement::BusAssign { name: name.clone(), expr: typed_expr })
            }
            Statement::Output { expr } => {
                let typed_expr = self.check_expr(expr)?;
                // Output should be numeric (audio signal)
                if typed_expr.ty != PatternType::Numeric && typed_expr.ty != PatternType::Unknown {
                    return Err(TypeError {
                        kind: TypeErrorKind::PatternTypeMismatch,
                        location: Some(typed_expr.span.clone()),
                        expected: "Numeric (audio signal)".into(),
                        got: format!("{:?}", typed_expr.ty),
                    });
                }
                Ok(TypedStatement::Output { expr: typed_expr })
            }
            // ... other statements
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<TypedExpr, TypeError> {
        match expr {
            Expr::Number(n) => Ok(TypedExpr {
                expr: expr.clone(),
                ty: PatternType::Numeric,
                span: SourceLocation::default(),
            }),

            Expr::String(s) => {
                // Could be sample pattern or numeric pattern
                if looks_like_numeric_pattern(s) {
                    Ok(TypedExpr { expr: expr.clone(), ty: PatternType::Numeric, span: SourceLocation::default() })
                } else {
                    Ok(TypedExpr { expr: expr.clone(), ty: PatternType::String, span: SourceLocation::default() })
                }
            }

            Expr::BusRef(name) => {
                if let Some(ty) = self.buses.get(name) {
                    Ok(TypedExpr { expr: expr.clone(), ty: ty.clone(), span: SourceLocation::default() })
                } else {
                    Err(TypeError {
                        kind: TypeErrorKind::PatternTypeMismatch,
                        location: None,
                        expected: "defined bus".into(),
                        got: format!("undefined bus '~{}'", name),
                    })
                }
            }

            Expr::BinOp { left, op, right } => {
                let left_typed = self.check_expr(left)?;
                let right_typed = self.check_expr(right)?;

                match op {
                    Op::Add | Op::Sub | Op::Mul | Op::Div => {
                        // Arithmetic requires numeric operands
                        if left_typed.ty == PatternType::String || right_typed.ty == PatternType::String {
                            return Err(TypeError {
                                kind: TypeErrorKind::ArithmeticOnStrings,
                                location: None,
                                expected: "numeric patterns".into(),
                                got: "string pattern in arithmetic".into(),
                            });
                        }
                        Ok(TypedExpr {
                            expr: expr.clone(),
                            ty: PatternType::Numeric,
                            span: SourceLocation::default(),
                        })
                    }
                    // ... other ops
                }
            }

            Expr::Call { name, args } => {
                self.check_function_call(name, args)
            }

            // ... other expression types
        }
    }

    fn check_function_call(&mut self, name: &str, args: &[Expr]) -> Result<TypedExpr, TypeError> {
        // Function signature lookup
        let sig = get_function_signature(name)?;

        // Arity check
        if args.len() != sig.params.len() {
            return Err(TypeError {
                kind: TypeErrorKind::PatternTypeMismatch,
                location: None,
                expected: format!("{} arguments", sig.params.len()),
                got: format!("{} arguments", args.len()),
            });
        }

        // Type check each argument
        for (i, (arg, param)) in args.iter().zip(sig.params.iter()).enumerate() {
            let arg_typed = self.check_expr(arg)?;
            if !types_compatible(&arg_typed.ty, &param.ty) {
                return Err(TypeError {
                    kind: TypeErrorKind::PatternTypeMismatch,
                    location: None,
                    expected: format!("{:?} for parameter '{}'", param.ty, param.name),
                    got: format!("{:?}", arg_typed.ty),
                });
            }
        }

        Ok(TypedExpr {
            expr: Expr::Call { name: name.into(), args: args.to_vec() },
            ty: sig.return_type,
            span: SourceLocation::default(),
        })
    }
}

fn looks_like_numeric_pattern(s: &str) -> bool {
    // "110 220 440" -> numeric
    // "bd sn hh" -> string
    s.split_whitespace()
        .all(|word| word.parse::<f64>().is_ok() || word == "~")
}
```

---

## Layer 3: Semantic Validation

### Parameter Range Validation

```rust
/// Musical parameter ranges
pub struct ParameterLimits {
    pub frequency_min: f64,   // 20 Hz
    pub frequency_max: f64,   // 20000 Hz
    pub gain_min: f64,        // 0.0
    pub gain_max: f64,        // 2.0 (allow some headroom)
    pub resonance_min: f64,   // 0.1
    pub resonance_max: f64,   // 20.0
    pub bpm_min: f64,         // 20 BPM
    pub bpm_max: f64,         // 999 BPM
    pub attack_min: f64,      // 0.0 seconds
    pub attack_max: f64,      // 10.0 seconds
    pub nesting_max: usize,   // 50 levels
}

impl Default for ParameterLimits {
    fn default() -> Self {
        Self {
            frequency_min: 20.0,
            frequency_max: 20000.0,
            gain_min: 0.0,
            gain_max: 2.0,
            resonance_min: 0.1,
            resonance_max: 20.0,
            bpm_min: 20.0,
            bpm_max: 999.0,
            attack_min: 0.0,
            attack_max: 10.0,
            nesting_max: 50,
        }
    }
}

/// Semantic validator for compiled expressions
pub struct SemanticValidator {
    limits: ParameterLimits,
    warnings: Vec<SemanticError>,
}

impl SemanticValidator {
    pub fn validate(&mut self, typed: &TypedExpr) -> Vec<SemanticError> {
        self.warnings.clear();
        self.check_expr(typed);
        std::mem::take(&mut self.warnings)
    }

    fn check_expr(&mut self, expr: &TypedExpr) {
        match &expr.expr {
            Expr::Call { name, args } => {
                self.check_function_params(name, args, &expr.span);
            }
            Expr::BinOp { left, right, .. } => {
                if let (Expr::BusRef(bus), _) | (_, Expr::BusRef(bus)) = (left.as_ref(), right.as_ref()) {
                    // Check for self-reference would go here
                }
            }
            _ => {}
        }
    }

    fn check_function_params(&mut self, name: &str, args: &[Expr], loc: &SourceLocation) {
        match name {
            "lpf" | "hpf" | "bpf" => {
                // First arg is cutoff frequency
                if let Some(Expr::Number(freq)) = args.first() {
                    if *freq < self.limits.frequency_min {
                        self.warnings.push(SemanticError {
                            kind: SemanticErrorKind::FrequencyOutOfRange {
                                freq: *freq,
                                min: self.limits.frequency_min,
                                max: self.limits.frequency_max,
                            },
                            location: Some(loc.clone()),
                            severity: Severity::Warning,
                        });
                    }
                    if *freq > self.limits.frequency_max {
                        self.warnings.push(SemanticError {
                            kind: SemanticErrorKind::FrequencyOutOfRange {
                                freq: *freq,
                                min: self.limits.frequency_min,
                                max: self.limits.frequency_max,
                            },
                            location: Some(loc.clone()),
                            severity: Severity::Warning,
                        });
                    }
                }
                // Second arg is resonance
                if let Some(Expr::Number(q)) = args.get(1) {
                    if *q < self.limits.resonance_min || *q > self.limits.resonance_max {
                        self.warnings.push(SemanticError {
                            kind: SemanticErrorKind::ResonanceOutOfRange {
                                q: *q,
                                min: self.limits.resonance_min,
                                max: self.limits.resonance_max,
                            },
                            location: Some(loc.clone()),
                            severity: Severity::Warning,
                        });
                    }
                }
            }

            "sine" | "saw" | "tri" | "square" => {
                // Frequency argument
                if let Some(Expr::Number(freq)) = args.first() {
                    if *freq < self.limits.frequency_min || *freq > self.limits.frequency_max {
                        self.warnings.push(SemanticError {
                            kind: SemanticErrorKind::FrequencyOutOfRange {
                                freq: *freq,
                                min: self.limits.frequency_min,
                                max: self.limits.frequency_max,
                            },
                            location: Some(loc.clone()),
                            severity: Severity::Warning,
                        });
                    }
                }
            }

            "adsr" => {
                // Attack, decay, sustain, release - all should be non-negative
                for (i, name) in ["attack", "decay", "sustain", "release"].iter().enumerate() {
                    if let Some(Expr::Number(val)) = args.get(i) {
                        if *val < 0.0 {
                            self.warnings.push(SemanticError {
                                kind: SemanticErrorKind::TimeNegative {
                                    param: name.to_string(),
                                    value: *val,
                                },
                                location: Some(loc.clone()),
                                severity: Severity::Error, // Negative time is an error
                            });
                        }
                    }
                }
            }

            _ => {}
        }
    }
}
```

### Dependency Cycle Detection

```rust
/// Check for cyclic dependencies between buses
pub fn check_cycles(buses: &HashMap<String, TypedExpr>) -> Result<(), SemanticError> {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    // Build dependency graph
    for (name, expr) in buses {
        let deps = collect_bus_refs(expr);
        graph.insert(name.clone(), deps);
    }

    // DFS for cycles
    for start in graph.keys() {
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        if has_cycle(&graph, start, &mut visited, &mut path) {
            return Err(SemanticError {
                kind: SemanticErrorKind::CyclicDependency {
                    chain: path,
                },
                location: None,
                severity: Severity::Error,
            });
        }
    }

    Ok(())
}

fn collect_bus_refs(expr: &TypedExpr) -> Vec<String> {
    let mut refs = Vec::new();
    collect_refs_recursive(&expr.expr, &mut refs);
    refs
}

fn collect_refs_recursive(expr: &Expr, refs: &mut Vec<String>) {
    match expr {
        Expr::BusRef(name) => refs.push(name.clone()),
        Expr::BinOp { left, right, .. } => {
            collect_refs_recursive(left, refs);
            collect_refs_recursive(right, refs);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                collect_refs_recursive(arg, refs);
            }
        }
        _ => {}
    }
}
```

---

## Layer 4: Runtime Validation

### NaN/Infinity Detection

```rust
/// Runtime validator for signal values
pub struct RuntimeValidator {
    error_count: AtomicUsize,
    last_error: Mutex<Option<RuntimeError>>,
}

impl RuntimeValidator {
    /// Check a signal value and return safe fallback if needed
    #[inline]
    pub fn validate_signal(&self, value: f64, context: &str, default: f64) -> f64 {
        if value.is_nan() {
            self.record_error(RuntimeError {
                kind: RuntimeErrorKind::NaN { operation: context.into() },
                context: context.into(),
                recovery: RecoveryAction::UseDefault(default),
            });
            default
        } else if value.is_infinite() {
            self.record_error(RuntimeError {
                kind: RuntimeErrorKind::Infinity { operation: context.into() },
                context: context.into(),
                recovery: RecoveryAction::UseDefault(default),
            });
            default
        } else {
            value
        }
    }

    /// Check division by zero
    #[inline]
    pub fn validate_division(&self, numerator: f64, denominator: f64) -> f64 {
        if denominator.abs() < f64::EPSILON {
            self.record_error(RuntimeError {
                kind: RuntimeErrorKind::DivisionByZero,
                context: format!("{} / {}", numerator, denominator),
                recovery: RecoveryAction::UseDefault(numerator),
            });
            numerator
        } else {
            numerator / denominator
        }
    }

    fn record_error(&self, error: RuntimeError) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut guard) = self.last_error.try_lock() {
            *guard = Some(error);
        }
    }

    pub fn has_errors(&self) -> bool {
        self.error_count.load(Ordering::Relaxed) > 0
    }

    pub fn error_count(&self) -> usize {
        self.error_count.load(Ordering::Relaxed)
    }

    pub fn take_last_error(&self) -> Option<RuntimeError> {
        self.last_error.try_lock().ok().and_then(|mut g| g.take())
    }
}
```

### Integration with Pattern Evaluation

```rust
// pattern.rs - Updated pattern arithmetic

impl Pattern<f64> {
    pub fn div_structure(self, other: Pattern<f64>, validator: &RuntimeValidator) -> Pattern<f64> {
        let left = self;
        let right = other;

        Pattern::new(Arc::new(move |state: &State| -> Vec<Hap<f64>> {
            let left_events = left.query(state);
            let right_events = right.query(state);

            let mut results = Vec::new();

            for mut hap in left_events {
                for other_hap in &right_events {
                    if hap.part.overlaps(&other_hap.part) {
                        hap.value = validator.validate_division(hap.value, other_hap.value);
                    }
                }
                results.push(hap);
            }

            results
        }))
    }
}
```

---

## Error Display and User Experience

### Terminal Display

```rust
impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "{}", format_parse_error(e)),
            Self::Type(e) => write!(f, "{}", format_type_error(e)),
            Self::Semantic(e) => write!(f, "{}", format_semantic_error(e)),
            Self::Runtime(e) => write!(f, "{}", format_runtime_error(e)),
        }
    }
}

fn format_parse_error(e: &ParseError) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!("❌ Parse Error at line {}:{}\n\n",
        e.location.line, e.location.column));

    // Source line with pointer
    out.push_str(&format!("  {}\n", e.source_line));
    out.push_str(&format!("  {}^\n\n", " ".repeat(e.location.column.saturating_sub(1))));

    // Error message
    out.push_str(&format!("Error: {}\n", e.kind.to_string()));

    // Hint
    if let Some(hint) = &e.hint {
        out.push_str(&format!("\n💡 Hint: {}\n", hint));
    }

    out
}
```

### Live Coding Integration

For `phonon-edit`, show errors inline:

```rust
/// Error overlay for live coding display
pub struct ErrorOverlay {
    errors: Vec<(usize, String)>,  // (line, message)
}

impl ErrorOverlay {
    pub fn from_validation_errors(errors: &[ValidationError]) -> Self {
        let mut overlay_errors = Vec::new();

        for err in errors {
            if let Some(loc) = err.location() {
                overlay_errors.push((loc.line, err.short_message()));
            }
        }

        Self { errors: overlay_errors }
    }

    pub fn render(&self, buffer: &mut Buffer, viewport_start: usize) {
        for (line, msg) in &self.errors {
            if *line >= viewport_start {
                let y = line - viewport_start;
                // Render error indicator in gutter
                buffer.set_string(0, y as u16, "⚠", Style::default().fg(Color::Red));
                // Could also render inline annotation
            }
        }
    }
}
```

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1)
**Goal**: Error types and basic parse-time validation

1. Create `src/validation.rs` with error type definitions
2. Update `mini_notation_v3.rs` to return `Result<Pattern<String>, ParseError>`
3. Add bracket validation
4. Add Euclidean pattern validation
5. Integrate with `error_diagnostics.rs`
6. Add 20+ unit tests for validation

**Deliverables**:
- `src/validation.rs` - Error types
- Updated `mini_notation_v3.rs`
- `tests/test_parse_validation.rs`

### Phase 2: Type Checking (Week 2)
**Goal**: Catch type mismatches at compile time

1. Implement `TypeChecker` struct
2. Add function signature registry
3. Type-check bus assignments
4. Type-check arithmetic operations
5. Add 30+ type checking tests

**Deliverables**:
- `src/type_checker.rs`
- `tests/test_type_checking.rs`

### Phase 3: Semantic Validation (Week 3)
**Goal**: Catch invalid parameters and patterns

1. Implement `SemanticValidator`
2. Add parameter range checking
3. Add dependency cycle detection
4. Add nesting depth limits
5. Add 25+ semantic validation tests

**Deliverables**:
- `src/semantic_validator.rs`
- `tests/test_semantic_validation.rs`

### Phase 4: Runtime Validation (Week 4)
**Goal**: Graceful handling of runtime errors

1. Implement `RuntimeValidator`
2. Integrate into pattern arithmetic
3. Integrate into signal evaluation
4. Add error logging/metrics
5. Add 20+ runtime validation tests

**Deliverables**:
- `src/runtime_validator.rs`
- Updated `pattern.rs`, `unified_graph.rs`
- `tests/test_runtime_validation.rs`

### Phase 5: Integration & Polish (Week 5)
**Goal**: Complete integration and user experience

1. Update `compositional_compiler.rs` to use all validators
2. Add error display to `phonon-edit`
3. Add validation to `phonon play`
4. Performance optimization (inline hot paths)
5. Documentation

**Deliverables**:
- Updated compiler with full validation pipeline
- Error display in phonon-edit
- Performance benchmarks
- User-facing documentation

---

## Success Criteria

### Quantitative
- [ ] 100+ validation-related tests passing
- [ ] All mini-notation edge cases handled (brackets, Euclidean, etc.)
- [ ] Zero silent failures (`unwrap_or` patterns eliminated)
- [ ] Error messages include line/column for all parse errors
- [ ] Type errors caught before runtime for 95%+ of cases

### Qualitative
- [ ] Users get helpful error messages with hints
- [ ] Errors include musical context ("did you mean...?")
- [ ] phonon-edit shows errors inline
- [ ] No more "why is my pattern silent?" debugging sessions
- [ ] Validation is fast enough for live coding (< 5ms)

---

## Testing Strategy

### Unit Tests
Each validation layer has dedicated tests:

```rust
// tests/test_parse_validation.rs

#[test]
fn test_unmatched_bracket_error() {
    let result = parse_mini_notation("bd [sn hh");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.kind, ParseErrorKind::UnmatchedBracket { .. }));
    assert!(err.hint.is_some());
}

#[test]
fn test_euclidean_validation() {
    // Valid
    assert!(parse_mini_notation("bd(3,8)").is_ok());

    // Invalid: more pulses than steps
    let result = parse_mini_notation("bd(10,8)");
    assert!(result.is_err());

    // Invalid: negative steps
    let result = parse_mini_notation("bd(3,-8)");
    assert!(result.is_err());
}

// tests/test_type_checking.rs

#[test]
fn test_arithmetic_type_error() {
    let code = r#"
        ~samples: s "bd sn"
        out: ~samples + 100  -- Can't add number to sample pattern
    "#;

    let result = typecheck_program(code);
    assert!(result.is_err());
}

// tests/test_semantic_validation.rs

#[test]
fn test_frequency_range_warning() {
    let code = "out: sine 50000";  // Above audible range
    let warnings = validate_semantics(code);
    assert!(!warnings.is_empty());
    assert!(matches!(
        warnings[0].kind,
        SemanticErrorKind::FrequencyOutOfRange { .. }
    ));
}

// tests/test_runtime_validation.rs

#[test]
fn test_division_by_zero_recovery() {
    let validator = RuntimeValidator::new();
    let result = validator.validate_division(100.0, 0.0);
    assert_eq!(result, 100.0);  // Returns numerator as fallback
    assert!(validator.has_errors());
}
```

### Integration Tests
End-to-end validation testing:

```rust
#[test]
fn test_full_validation_pipeline() {
    let code = r#"
        bpm: 120
        ~drums: s "bd [sn hh]"
        ~bass: saw 55
        out: ~drums + ~bass * 0.3
    "#;

    // Should pass all validation layers
    let result = compile_with_validation(code);
    assert!(result.is_ok());
}

#[test]
fn test_validation_catches_all_error_types() {
    // Parse error
    assert!(compile_with_validation("out: s \"bd[\"").is_err());

    // Type error (if we implement strict typing)
    // assert!(compile_with_validation("out: \"bd\" + 5").is_err());

    // Semantic error
    let result = compile_with_validation("out: sine -50");  // Negative frequency
    assert!(result.warnings().len() > 0);
}
```

---

## Compatibility Notes

### Migration Path

1. **Phase 1**: New validation functions alongside existing code
2. **Phase 2**: Opt-in validation via compiler flag
3. **Phase 3**: Default to validation with deprecation warnings
4. **Phase 4**: Full validation, old behavior removed

### Backwards Compatibility

The validation framework adds new error paths but doesn't change the semantics of valid programs:
- Valid mini-notation still produces same patterns
- Valid DSL still compiles to same signal graphs
- Only invalid input behavior changes (from silent to loud)

---

## Future Enhancements

### IDE Integration (LSP)
The error types are designed for IDE integration:
- `SourceLocation` provides range information
- Error kinds can map to diagnostic codes
- Hints can become quick-fix suggestions

### Error Recovery
Future work could add error recovery to continue parsing:
- Insert missing brackets
- Suggest corrections for typos
- Parse remaining valid statements

### Configurable Strictness
Allow users to configure validation strictness:
```phonon
-- Strict mode (default): all warnings are errors
-- Relaxed mode: warnings don't stop compilation
validation: relaxed
```

---

## Appendix: Current Silent Failures

### In mini_notation_v3.rs
- Empty input → `Pattern::silence()`
- Parse failure → `Pattern::silence()`
- Invalid Euclidean → `Pattern::silence()`

### In compositional_compiler.rs
- Failed float parse → `unwrap_or(1.0)` (lines 602, 8043, 8072, 8100, 8116)
- Unknown transform → continues with partial result
- Missing bus → may panic or return default

### In pattern.rs
- Division by zero → returns original value (silent)
- Empty `choose([])` → silence
- Empty `run(0)` → silence

All of these will be replaced with explicit error handling.
