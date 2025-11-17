# Phonon Autocomplete Improvements Plan

## Current State

### Metadata System (Already Exists!)
- **96 functions** manually documented in `function_metadata.rs`
- **143 functions** actually in compiler → **47 missing**
- Metadata includes: name, description, params (with kwargs), examples, categories
- Already shows `:keyword` syntax in signatures

### Problems
1. ❌ Metadata manually maintained and out of date (47 functions behind)
2. ❌ Parser doesn't support `:keyword value` syntax
3. ❌ No context-aware completion for kwargs inside function calls

---

## Solution Architecture

### 1. Auto-Generate Metadata from Compiler

**Option A: Doc Comments + Macro (Recommended)**
```rust
/// Low-pass filter - removes frequencies above cutoff
///
/// # Example
/// ```phonon
/// ~bass: saw 55 # lpf 800 :q 1.5
/// ```
#[phonon_function(category = "Filters")]
fn compile_lpf(
    ctx: &mut CompilerContext,
    #[param(desc = "Filter cutoff frequency in Hz")] cutoff: Expr,
    #[param(desc = "Filter resonance/Q factor (0.1-10)", default = "1.0")] q: Option<Expr>,
) -> Result<NodeId, String> {
    // ...
}
```

**Option B: Declarative Registry (Simpler, Less Invasive)**
```rust
// In compositional_compiler.rs
register_functions! {
    lpf(cutoff: Hz, q: float = 1.0) => compile_lpf {
        category: "Filters",
        description: "Low-pass filter - removes frequencies above cutoff",
        example: "~bass: saw 55 # lpf 800 :q 1.5",
    }

    hpf(cutoff: Hz, q: float = 1.0) => compile_hpf {
        category: "Filters",
        description: "High-pass filter - removes frequencies below cutoff",
        example: "~lead: saw 440 # hpf 200 :q 0.8",
    }

    // ... all 143 functions
}
```

This macro would:
1. Generate the function dispatch table
2. Auto-generate `FUNCTION_METADATA`
3. Single source of truth!

---

### 2. Parser Support for `:keyword` Syntax

**AST Changes:**
```rust
// In compositional_parser.rs
#[derive(Debug, Clone)]
pub enum Expr {
    // ... existing variants

    /// Keyword argument: :name value
    Kwarg {
        name: String,
        value: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub struct Call {
    pub name: String,
    /// Positional arguments
    pub args: Vec<Expr>,
    /// Keyword arguments (new!)
    pub kwargs: HashMap<String, Expr>,
}
```

**Parser Implementation:**
```rust
// Parse keyword argument: :name value
fn parse_kwarg(input: &str) -> IResult<&str, Expr> {
    map(
        tuple((
            preceded(tag(":"), parse_identifier),
            preceded(space0, parse_primary_expr),
        )),
        |(name, value)| Expr::Kwarg {
            name,
            value: Box::new(value),
        },
    )(input)
}

// Update function call parser
fn parse_function_call(input: &str) -> IResult<&str, Expr> {
    let (input, name) = parse_identifier(input)?;
    let (input, _) = space0(input)?;

    let mut positional = Vec::new();
    let mut kwargs = HashMap::new();

    // Parse args until end
    let mut remaining = input;
    while !remaining.is_empty() {
        // Try kwarg first
        if let Ok((rest, Expr::Kwarg { name, value })) = parse_kwarg(remaining) {
            kwargs.insert(name, *value);
            remaining = rest;
        }
        // Then positional
        else if let Ok((rest, expr)) = parse_primary_expr(remaining) {
            positional.push(expr);
            remaining = rest;
        }
        else {
            break;
        }

        // Optional separator
        if let Ok((rest, _)) = space0(remaining) {
            remaining = rest;
        }
    }

    Ok((remaining, Expr::Call { name, args: positional, kwargs }))
}
```

**Compiler Changes:**
```rust
fn compile_lpf(ctx: &mut CompilerContext, args: Vec<Expr>, kwargs: HashMap<String, Expr>)
    -> Result<NodeId, String>
{
    // Get cutoff from positional or kwargs
    let cutoff = args.get(0)
        .or_else(|| kwargs.get("cutoff"))
        .ok_or("lpf requires cutoff parameter")?;

    // Get q from positional, kwargs, or default
    let q = args.get(1)
        .or_else(|| kwargs.get("q"))
        .unwrap_or(&Expr::Number(1.0));

    // ... compile
}
```

---

### 3. Context-Aware Autocomplete

**Detect Function Context:**
```rust
// In completion/context.rs

#[derive(Debug)]
pub enum CompletionContext {
    /// Top-level, show all functions
    TopLevel,

    /// Inside function call, show kwargs
    InsideFunctionCall {
        function_name: String,
        /// Already provided kwargs
        provided_kwargs: HashSet<String>,
        /// Cursor after positional arg N
        after_positional: usize,
    },

    /// After transform operator $
    AfterTransform,
}

pub fn get_completion_context(text: &str, cursor: usize) -> CompletionContext {
    // Parse from start to cursor
    let before_cursor = &text[..cursor];

    // Find last unclosed function call
    if let Some(func_name) = find_enclosing_function(before_cursor) {
        let provided = parse_provided_kwargs(before_cursor, &func_name);
        let positional_count = count_positional_args(before_cursor, &func_name);

        return CompletionContext::InsideFunctionCall {
            function_name: func_name,
            provided_kwargs: provided,
            after_positional: positional_count,
        };
    }

    CompletionContext::TopLevel
}
```

**Generate Context-Aware Completions:**
```rust
pub fn get_completions(text: &str, cursor: usize) -> Vec<Completion> {
    let context = get_completion_context(text, cursor);

    match context {
        CompletionContext::TopLevel => {
            // Show all functions from FUNCTION_METADATA
            FUNCTION_METADATA.values()
                .map(|meta| Completion {
                    label: meta.name.to_string(),
                    detail: Some(meta.description.to_string()),
                    signature: Some(meta.param_signature()),
                    kind: CompletionKind::Function,
                })
                .collect()
        }

        CompletionContext::InsideFunctionCall { function_name, provided_kwargs, .. } => {
            // Show kwargs for this function that haven't been provided yet
            if let Some(meta) = FUNCTION_METADATA.get(function_name.as_str()) {
                meta.params.iter()
                    .filter(|param| !provided_kwargs.contains(param.name))
                    .map(|param| Completion {
                        label: format!(":{}", param.name),
                        detail: Some(param.description.to_string()),
                        signature: Some(format!("{} ({})", param.param_type,
                            if param.optional { "optional" } else { "required" })),
                        kind: CompletionKind::Kwarg,
                    })
                    .collect()
            } else {
                vec![]
            }
        }

        CompletionContext::AfterTransform => {
            // Show transform functions (fast, slow, etc.)
            get_transform_completions()
        }
    }
}
```

---

## Implementation Plan

### Phase 1: Parser Support for `:keyword` (2-3 hours)
1. Add `Kwarg` variant to `Expr` enum
2. Add `kwargs: HashMap<String, Expr>` to `Call` struct
3. Implement `parse_kwarg()` parser
4. Update `parse_function_call()` to collect kwargs
5. Update ALL `compile_*` functions to accept `kwargs` parameter
6. Make compiler use kwargs with fallback to positional

### Phase 2: Auto-Generate Metadata (3-4 hours)
1. Design `register_functions!` macro syntax
2. Implement macro to generate:
   - Function dispatch table
   - `FUNCTION_METADATA` HashMap
3. Convert existing function registrations to macro format
4. Add all 143 functions (copy from compiler, add descriptions)

### Phase 3: Context-Aware Completions (2-3 hours)
1. Implement `get_completion_context()` with function detection
2. Update completion generator to use context
3. Add kwarg filtering (hide already-provided)
4. Test in modal editor

**Total: ~8-10 hours of work**

---

## Benefits

✅ **No more manual maintenance** - Single source of truth
✅ **Always up to date** - Metadata generated from actual compiler
✅ **Discoverable** - See all kwargs via autocomplete
✅ **Readable** - Use `:cutoff 800` instead of positional args
✅ **Flexible** - Mix positional and kwargs: `lpf 800 :q 1.5`
✅ **Self-documenting** - Autocomplete shows types, defaults, descriptions

## Example Usage After Implementation

```phonon
# Autocomplete shows: lpf(cutoff: Hz, q: float = 1.0)
~bass: saw 55 # lpf <TAB>
# Shows: :cutoff :q

~bass: saw 55 # lpf 800 <TAB>
# Shows: :q (cutoff already provided)

~bass: saw 55 # lpf :cutoff 800 :q 1.5  # Explicit kwargs
~bass: saw 55 # lpf 800 1.5              # Positional still works
~bass: saw 55 # lpf 800 :q 1.5           # Mixed!
```
