# Tab Completion Implementation Decisions

## Status: Ready for Decision

This document outlines the key decisions needed before implementing tab completion. Each decision has options with recommendations based on common editor patterns.

---

## DECISION 1: Trigger Behavior

**Question**: When should completion be triggered?

### Options:

**A. Tab-only (Recommended)**
- User presses Tab to show completions
- Explicit, predictable, low-overhead

**B. Auto-trigger after typing**
- Show after N characters (e.g., 2-3)
- More "modern" but can be distracting
- Higher performance requirements

**C. Hybrid**
- Tab always works
- Auto-trigger in specific contexts (e.g., after `s "`)

**Recommendation**: **A (Tab-only)**
- Simpler to implement
- Lower performance requirements
- Common in terminal editors (vim, emacs)
- User has explicit control

---

## DECISION 2: Tab Behavior When Popup Visible

**Question**: What does Tab do when completion popup is already showing?

### Options:

**A. Navigate to next item** (like Ctrl+N)
- Tab cycles through completions
- Enter accepts selected

**B. Dismiss popup**
- Tab closes popup without applying
- Must use Enter to accept

**C. Accept and show next** (Recommended)
- Tab accepts current selection
- If more matches, shows completions again

**Recommendation**: **C (Accept and show next)**
- Allows rapid Tab-Tab-Tab completion flow
- Common in IDEs (IntelliJ, VSCode)
- Most efficient for power users

---

## DECISION 3: Popup Placement

**Question**: Where should the completion popup appear?

### Options:

**A. Below cursor line** (Recommended)
- Natural reading flow
- Issue: Might go off-screen at bottom

**B. Above cursor line**
- Avoids off-screen at bottom
- Issue: Less natural, blocks context

**C. Smart (below if room, above if not)**
- Best of both worlds
- More complex implementation

**Recommendation**: **C (Smart placement)**
- Better UX in all situations
- Not much more complex than fixed placement

---

## DECISION 4: Maximum Visible Completions

**Question**: How many completions to show in popup?

### Options:

**A. Show all** (up to terminal height)
- User sees everything
- Issue: Can be overwhelming

**B. Fixed limit (e.g., 10)** (Recommended)
- Clean, focused UI
- Scroll with arrows if more matches

**C. Adaptive (more if space available)**
- Uses available space
- More complex

**Recommendation**: **B (Fixed limit of 10)**
- Standard in most editors
- Good balance of visibility and focus
- Simple to implement

---

## DECISION 5: Keyboard Navigation

**Question**: What keys navigate the completion popup?

### Options:

**A. Arrows only** (Recommended)
- Up/Down: Navigate
- Enter: Accept
- Escape: Dismiss

**B. Arrows + Ctrl shortcuts**
- Ctrl+N/P: Navigate (vim/emacs style)
- Ctrl+Y: Accept (vim style)
- More shortcuts for power users

**C. Tab navigation**
- Tab/Shift+Tab to navigate
- Enter to accept

**Recommendation**: **A (Arrows only)**
- Most intuitive for new users
- Can add Ctrl shortcuts later if requested
- Keep it simple for v1

---

## DECISION 6: Sample Discovery Timing

**Question**: When to scan ~/dirt-samples/ for sample names?

### Options:

**A. At editor startup** (Recommended)
- One-time cost (~10-50ms)
- Completions available immediately
- Issue: Startup delay

**B. Lazy on first Tab**
- No startup cost
- Issue: First Tab press has delay

**C. Background thread at startup**
- Non-blocking startup
- More complex (threading, sync)

**Recommendation**: **A (At startup)**
- Acceptable delay for typical case
- Simpler implementation
- Can optimize later if needed

---

## DECISION 7: Bus Name Extraction Timing

**Question**: When to re-scan content for ~bus: definitions?

### Options:

**A. After every keystroke** (Recommended)
- Always up-to-date
- Fast enough (~100µs for 100 lines)

**B. Debounced (e.g., 200ms after typing stops)**
- Lower CPU usage
- Issue: Outdated during fast typing

**C. Only on Tab press**
- Minimal overhead
- Issue: Can be outdated

**Recommendation**: **A (After every keystroke)**
- Real-time accuracy
- Performance is acceptable
- Simpler implementation (no debounce timer)

---

## DECISION 8: Completion Sorting

**Question**: How to order completions within each type?

### Options:

**A. Alphabetical only** (Recommended)
- Predictable, deterministic
- Easy to find items

**B. By frequency/usage**
- Most-used items first
- Issue: Need to track usage stats

**C. Hybrid (frecency: frequency + recency)**
- Like browser history
- More complex

**Recommendation**: **A (Alphabetical only)**
- Simple and predictable for v1
- Can add frequency later if requested

---

## DECISION 9: No Matches Behavior

**Question**: What to do when no completions match?

### Options:

**A. Show nothing** (Recommended)
- Silent failure
- User knows Tab didn't work

**B. Show "No matches" message**
- Explicit feedback
- Issue: Can be noisy

**C. Show all available (ignore partial)**
- Fall back to showing everything
- Issue: Confusing

**Recommendation**: **A (Show nothing)**
- Clean UX
- Matches common editor behavior
- Can show message in status line if needed

---

## DECISION 10: Module Organization

**Question**: How to organize the completion code?

### Options:

**A. Single file: completion.rs**
- All logic in one place
- Issue: Large file (~500+ lines)

**B. Module directory** (Recommended)
```
src/modal_editor/
  mod.rs              // Main editor
  highlighting.rs     // Syntax highlighting
  completion/
    mod.rs            // Public API
    context.rs        // Context detection
    matching.rs       // Filtering and matching
    state.rs          // Popup state machine
    discovery.rs      // Sample/bus discovery
```

**C. Inline in modal_editor.rs**
- Keep everything together
- Issue: File already 1600+ lines

**Recommendation**: **B (Module directory)**
- Clear separation of concerns
- Easier to test each part
- Room for growth

---

## DECISION 11: Completion Type Labels

**Question**: How to show type labels in popup?

### Options:

**A. Color-coded only**
- Blue for functions, white for samples, magenta for buses
- Issue: Not obvious, colorblind issues

**B. Text labels** (Recommended)
```
bd          [sample]
~bass       [bus]
fast        [function]
```

**C. Icons/Symbols**
```
bd          📁
~bass       🔗
fast        ⚡
```

**Recommendation**: **B (Text labels)**
- Clear and accessible
- No ambiguity
- Works in all terminals

---

## DECISION 12: Escape Key Behavior

**Question**: What does Escape do when completion popup is visible?

### Options:

**A. Dismiss popup, stay in insert mode** (Recommended)
- Closes completion
- Cursor stays in same position
- Continue editing

**B. Dismiss popup AND exit to normal mode**
- Like vim behavior
- Issue: Unexpected for non-vim users

**Recommendation**: **A (Dismiss only)**
- Less surprising behavior
- More forgiving
- Can still press Escape again for normal mode

---

## Summary of Recommendations

| Decision | Recommendation | Rationale |
|----------|---------------|-----------|
| 1. Trigger | Tab-only | Simple, predictable |
| 2. Tab when visible | Accept and show next | Efficient workflow |
| 3. Placement | Smart (below/above) | Best UX |
| 4. Max visible | 10 items | Standard, focused |
| 5. Navigation | Arrows + Enter/Esc | Intuitive |
| 6. Sample discovery | At startup | Simple, fast enough |
| 7. Bus extraction | Every keystroke | Real-time, fast enough |
| 8. Sorting | Alphabetical | Predictable |
| 9. No matches | Show nothing | Clean |
| 10. Organization | Module directory | Maintainable |
| 11. Type labels | Text labels | Clear |
| 12. Escape | Dismiss only | Forgiving |

---

## Next Steps

1. **Get sign-off on decisions** from user
2. **Create GitHub issue** with approved decisions
3. **Implement in phases** per MODAL_EDITOR_TESTING_STRATEGY.md:
   - Phase 1: Pure functions + tests
   - Phase 2: State machine + tests
   - Phase 3: UI integration
   - Phase 4: Manual testing

---

## Questions for User

Before proceeding, please confirm:

1. ✅ **Context-aware with labels** (already confirmed)
2. ❓ **Tab-only trigger** or auto-trigger?
3. ❓ **10 items max** in popup okay?
4. ❓ **Any strong preferences** on other decisions?

---

## Alternative Approaches

### Minimal v1 (2-3 days)
- Tab-only trigger
- Show all completions (no limit)
- Alphabetical sort
- Simple below-cursor placement
- Basic navigation

### Full v1 (1-2 weeks)
- All recommendations above
- Comprehensive testing
- Smart placement
- Polished UX

Which approach should we take?
