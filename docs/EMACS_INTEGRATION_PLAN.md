# Emacs Integration for Phonon Live Coding

## Overview

Enable live coding with Phonon using Emacs, inspired by TidalCycles' tidal.el but adapted for Phonon's architecture and avoiding OSC complexity.

## Block-Based Execution Model

### Block Definition
- **Blocks** are contiguous groups of lines separated by blank lines
- Follows TidalCycles convention for familiar UX
- Each block can be executed independently
- Example:
```phonon
# Block 1: Drums
tempo 2.0
~drums s "bd sn hh*4 cp"
out1 ~drums

# Block 2: Bass (separate, can coexist)
~bass saw("55 82.5") # lpf(2000, 0.8)
out2 ~bass * 0.5
```

### Visual Feedback
- Temporary highlight on execution (using `pulse-momentary-highlight-region`)
- Subtle left border/background to show block boundaries
- Discoverable: user can see which lines will execute together

## Communication Architecture

### Recommendation: OSC (Open Sound Control)

After evaluating file-based, TCP, and OSC approaches, **OSC is recommended** for the following reasons:

**Why OSC over File-Based:**
1. **Fast iteration**: OSC messages are instant, no file watching latency
2. **Error feedback**: Can send errors back to editor (e.g., "syntax error on line 5")
3. **Ecosystem compatibility**: TidalCycles ecosystem expects OSC, easier for users to integrate
4. **Block execution**: Each block = separate OSC message with channel number
5. **Status queries**: Can query engine state (active voices, CPU usage, etc.)

**Why OSC Isn't As Complex As It Seems:**
- TidalCycles' complexity came from the Haskell/GHCi interpreter layer, not OSC itself
- Phonon is self-contained (no interpreter), so OSC is straightforward
- Modern OSC libraries are simple and reliable
- Standard port 7770 for live coding (no conflicts)

**OSC Message Format:**
```
/eval <channel:int> <code:string>    # Evaluate code on output channel
/hush                                # Silence all outputs
/hush <channel:int>                  # Silence specific channel
/panic                               # Kill all voices + silence
/status                              # Query engine status
```

**Implementation (Emacs side):**
```elisp
(require 'osc)

(defvar phonon-osc-client nil
  "OSC client connection to Phonon")

(defun phonon-connect ()
  "Connect to Phonon on localhost:7770"
  (setq phonon-osc-client (osc-make-client "localhost" 7770)))

(defun phonon-eval-block ()
  "Send current block to Phonon via OSC"
  (interactive)
  (let ((block-text (phonon-get-block-at-point))
        (channel (phonon-get-block-channel)))  ; Determine channel from position
    (osc-send-message phonon-osc-client "/eval" channel block-text)
    (pulse-momentary-highlight-region (region-beginning) (region-end))))

(defun phonon-hush (&optional channel)
  "Silence output channel(s)"
  (interactive "P")
  (if channel
      (osc-send-message phonon-osc-client "/hush" channel)
    (osc-send-message phonon-osc-client "/hush")))

(defun phonon-panic ()
  "Emergency stop: kill all voices"
  (interactive)
  (osc-send-message phonon-osc-client "/panic"))
```

**Implementation (Phonon side):**
- Use `rosc` crate for OSC server
- Listen on port 7770
- Handle `/eval`, `/hush`, `/panic`, `/status` messages
- Parse code and update corresponding output channel
- Send error messages back via `/error` message

### Alternative: File-Based
**Pros:**
- Can get status/error feedback
- More interactive feel
- Easier than OSC

**Cons:**
- Need to manage connection
- Port conflicts possible
- More complexity

**Implementation:**
```elisp
(defvar phonon-connection nil
  "TCP connection to Phonon")

(defun phonon-connect ()
  "Connect to Phonon on localhost:7770"
  (setq phonon-connection
        (make-network-process
         :name "phonon"
         :host "localhost"
         :service 7770
         :filter 'phonon-filter)))

(defun phonon-eval-block ()
  "Send current block to Phonon"
  (let ((block-text (phonon-get-block-at-point)))
    (process-send-string phonon-connection
                        (format "%d\n%s\n"
                               (length block-text)
                               block-text))
    (pulse-momentary-highlight-region (point-min) (point-max))))
```

## Key Emacs Functions

### Block Detection
```elisp
(defun phonon-get-block-at-point ()
  "Get the current code block (paragraph)"
  (save-excursion
    (mark-paragraph)
    (buffer-substring-no-properties (region-beginning) (region-end))))
```

### Keybindings
```elisp
(define-key phonon-mode-map (kbd "C-c C-c") 'phonon-eval-block)
(define-key phonon-mode-map (kbd "C-c C-a") 'phonon-eval-buffer)
(define-key phonon-mode-map (kbd "C-c C-h") 'phonon-hush)
(define-key phonon-mode-map (kbd "C-c C-p") 'phonon-panic)
```

### Visual Block Indication
```elisp
(defun phonon-highlight-blocks ()
  "Add subtle visual indication of block boundaries"
  (save-excursion
    (goto-char (point-min))
    (while (not (eobp))
      (let ((block-start (point)))
        (forward-paragraph)
        (let ((block-end (point)))
          (when (> (- block-end block-start) 1)
            ;; Add left border overlay
            (let ((ov (make-overlay block-start block-end)))
              (overlay-put ov 'before-string
                          (propertize " " 'display
                                    '(left-fringe vertical-bar))))))
        (forward-line 1)))))
```

## Multi-Output Integration

### Channel Mapping
- Each block can write to specific output channels
- Blocks execute independently
- Can remix by editing different blocks
- Example:
```phonon
# Block 1 → out1
~drums s "bd sn"
out1 ~drums

# Block 2 → out2
~bass saw(55)
out2 ~bass

# Block 3 → out3 (combines 1+2)
out3 (~drums + ~bass) * 0.5
```

### Hush/Panic
- `C-c C-h` → Silence all outputs
- `C-c C-p` → Kill all voices + silence
- Can hush specific channels: `(phonon-hush 1)` → silence channel 1

## Syntax Highlighting

```elisp
(setq phonon-font-lock-keywords
  '(("\\(~[a-zA-Z_][a-zA-Z0-9_]*\\)" . font-lock-variable-name-face)
    ("\\(out[0-9]*\\|tempo\\|cps\\)" . font-lock-keyword-face)
    ("\\(s\\|sine\\|saw\\|square\\|lpf\\|hpf\\)" . font-lock-function-name-face)
    ("#.*$" . font-lock-comment-face)
    ("\"[^\"]*\"" . font-lock-string-face)))
```

## Architecture: Master Bus and Output Routing

### Master Bus Concept

**Smart Auto-Routing**

Phonon automatically routes channels to master based on naming patterns, eliminating boilerplate while maintaining flexibility:

**Auto-routing patterns:**
- `~d1`, `~d2`, `~d3`, ... (TidalCycles style)
- `~out1`, `~out2`, `~out3`, ...

**Example - TidalCycles style (auto-routing):**
```phonon
cps: 2.0
~d1: saw 110        # Auto-routes to master
~d2: saw 220        # Auto-routes to master
~d3: saw 440        # Auto-routes to master
# All d-channels auto-sum → output
```

**Example - Named channels (manual control):**
```phonon
cps: 2.0
~drums: saw 110     # Named bus, does NOT auto-route
~bass: saw 55       # Named bus, does NOT auto-route
~master: ~drums + ~bass  # Explicit routing
```

**Example - Explicit master control:**
```phonon
tempo 2.0
~drums: s("bd sn hh*4 cp")
~bass: saw("55 82.5") # lpf(2000, 0.8)
~master: ~drums * 0.8 + ~bass * 0.5  # Explicit mix
```

**Example - Master processing:**
```phonon
~drums: s("bd*4")
~bass: saw("55 82.5")
~master: (~drums + ~bass) # lpf(5000, 0.5) # reverb(0.3, 0.8, 0.2)
# All audio goes through filter and reverb
```

**Example - Send/aux routing:**
```phonon
~drums: s("bd sn")
~reverb_send: ~drums # reverb(0.8, 0.5, 0.3)
~master: ~drums * 0.7 + ~reverb_send * 0.3  # Dry/wet mix
```

### Multi-Output for Hardware

For multi-channel audio interfaces, use numbered outputs:
```phonon
~drums: s("bd*4")
~bass: saw("55 82.5")

# Send to separate hardware channels
~out1: ~drums        # Output 1 (left monitor)
~out2: ~bass         # Output 2 (right monitor)
~out3: ~drums + ~bass  # Output 3 (main mix)
```

This allows live coding with multiple outputs while maintaining the master bus for the default stereo output.

## Recommended Implementation Phases

**Phase 1: Core Architecture**
- Implement `~master` bus with auto-sum behavior
- OSC server on port 7770
- `/eval`, `/hush`, `/panic` message handlers
- Block-based DSL parsing

**Phase 2: Emacs Integration**
- Basic phonon-mode with syntax highlighting
- Block detection and execution (OSC-based)
- Visual feedback (pulse highlighting)
- Hush/panic commands

**Phase 3: Advanced Features**
- Block visual indicators (left border)
- Error feedback in minibuffer
- Status line showing active voices/channels
- Multi-file support

## Differences from Tidal

1. **No Haskell REPL** - Phonon is self-contained, not through interpreter
2. **Simpler communication** - Files or TCP instead of OSC
3. **Multi-output** - Each block can target specific channels
4. **Real-time** - No compilation step, instant execution
5. **Visual blocks** - More discoverable than Tidal's invisible paragraph boundaries
