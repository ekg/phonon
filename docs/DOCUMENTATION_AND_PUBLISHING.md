# Documentation and Publishing Guide

This guide covers how to generate and view Rust documentation for Phonon, as well as the steps needed to publish to crates.io.

---

## Rendering Documentation

### Generate HTML Documentation

Rust's `cargo doc` command generates HTML documentation from your source code comments:

```bash
# Generate documentation for Phonon and all dependencies
cargo doc

# Generate only Phonon's documentation (faster)
cargo doc --no-deps

# Open documentation in your browser automatically
cargo doc --no-deps --open
```

The generated documentation will be in `target/doc/phonon/index.html`.

### Viewing Documentation Locally

```bash
# Option 1: Use cargo doc --open
cargo doc --no-deps --open

# Option 2: Open manually
firefox target/doc/phonon/index.html
# Or: google-chrome target/doc/phonon/index.html
# Or: open target/doc/phonon/index.html  # macOS

# Option 3: Serve with a local web server
cd target/doc
python3 -m http.server 8000
# Then visit http://localhost:8000/phonon/
```

### Documentation Comment Syntax

Rust uses special comment syntax for documentation:

```rust
//! Module-level documentation (goes at the top of the file)

/// Function/struct/enum documentation
pub fn example() {
    // Regular comments (not included in docs)
}
```

**Example with code blocks:**

```rust
/// Triggers a sample with gain, pan, and speed control.
///
/// # Examples
///
/// ```
/// use phonon::voice_manager::Voice;
/// use std::sync::Arc;
///
/// let mut voice = Voice::new();
/// let sample_data = vec![0.5, 0.6, 0.7];
/// let sample = Arc::new(sample_data);
///
/// // Play at normal speed with center pan
/// voice.trigger_with_speed(sample, 1.0, 0.0, 1.0);
/// ```
///
/// # Parameters
///
/// - `sample`: Audio sample data to play
/// - `gain`: Volume (0.0 to 1.0+)
/// - `pan`: Stereo position (-1.0 left, 0.0 center, 1.0 right)
/// - `speed`: Playback speed (1.0 normal, 2.0 double, 0.5 half)
pub fn trigger_with_speed(&mut self, sample: Arc<Vec<f32>>, gain: f32, pan: f32, speed: f32) {
    // Implementation...
}
```

### Testing Documentation Examples

Rust automatically runs code examples in documentation as tests:

```bash
# Run all tests including doc tests
cargo test

# Run only doc tests
cargo test --doc

# Run doc tests for a specific module
cargo test --doc voice_manager
```

---

## Publishing to Crates.io

### Prerequisites

Before publishing to crates.io, you need:

1. **A crates.io account**
   ```bash
   # Visit https://crates.io/ and sign in with GitHub

   # Get your API token from https://crates.io/me
   # Then login with cargo:
   cargo login <your-api-token>
   ```

2. **A LICENSE file**

   The README says "MIT" but there's no LICENSE file. Create one:

   ```bash
   # Create MIT license file
   cat > LICENSE << 'EOF'
   MIT License

   Copyright (c) 2025 Erik Garrison

   Permission is hereby granted, free of charge, to any person obtaining a copy
   of this software and associated documentation files (the "Software"), to deal
   in the Software without restriction, including without limitation the rights
   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
   copies of the Software, and to permit persons to whom the Software is
   furnished to do so, subject to the following conditions:

   The above copyright notice and this permission notice shall be included in all
   copies or substantial portions of the Software.

   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
   SOFTWARE.
   EOF
   ```

3. **Complete Cargo.toml metadata**

   Your current Cargo.toml needs these additions:

   ```toml
   [package]
   name = "phonon"
   version = "0.1.0"
   edition = "2021"
   authors = ["Erik Garrison <erik.garrison@gmail.com>"]
   description = "Phonon: A Rust-based live coding language combining TidalCycles patterns with modular synthesis"

   # ADD THESE FIELDS:
   license = "MIT"
   repository = "https://github.com/erikgarrison/phonon"
   homepage = "https://github.com/erikgarrison/phonon"
   documentation = "https://docs.rs/phonon"
   readme = "README.md"
   keywords = ["audio", "synthesis", "live-coding", "music", "dsp"]
   categories = ["multimedia::audio", "multimedia::encoding"]

   # Optional but recommended:
   exclude = [
       "samples/*",      # Don't upload sample files
       "*.ph",           # Don't upload phonon scripts
       "demos/*.ph",
       "examples/*.ph",
       "target/*",
   ]
   ```

   **Available categories** (see https://crates.io/category_slugs):
   - `multimedia::audio`
   - `multimedia::encoding`
   - `development-tools`
   - `algorithms`
   - `science`

4. **README.md** (✅ Already exists)

5. **Clean build and tests passing**

   ```bash
   # Ensure everything builds
   cargo build --release

   # Ensure all tests pass
   cargo test

   # Check for issues before publishing
   cargo publish --dry-run
   ```

### Publishing Steps

1. **Verify package contents**

   ```bash
   # Dry run to see what would be published
   cargo publish --dry-run

   # Create a package file to inspect
   cargo package

   # Inspect the package
   tar -tzf target/package/phonon-0.1.0.crate
   ```

2. **Publish to crates.io**

   ```bash
   # First release (version 0.1.0)
   cargo publish
   ```

3. **Update version for next release**

   ```bash
   # Edit Cargo.toml, bump version to 0.1.1 or 0.2.0
   # Then:
   cargo publish
   ```

### Version Numbering (SemVer)

Follow semantic versioning: `MAJOR.MINOR.PATCH`

- **MAJOR**: Breaking changes (e.g., 0.1.0 → 1.0.0)
- **MINOR**: New features, backward compatible (e.g., 0.1.0 → 0.2.0)
- **PATCH**: Bug fixes (e.g., 0.1.0 → 0.1.1)

**For pre-1.0 versions (like 0.1.0):**
- Breaking changes: bump MINOR (0.1.0 → 0.2.0)
- New features: bump PATCH (0.1.0 → 0.1.1)

### Publishing Checklist

Before running `cargo publish`:

- [ ] LICENSE file exists
- [ ] Cargo.toml has all required metadata
- [ ] README.md is up to date
- [ ] All tests pass (`cargo test`)
- [ ] Documentation builds (`cargo doc --no-deps`)
- [ ] No TODO or FIXME in critical paths
- [ ] Version number is correct
- [ ] Git tag created: `git tag v0.1.0 && git push --tags`
- [ ] Dry run successful: `cargo publish --dry-run`

---

## Automatic Documentation Hosting

### docs.rs

When you publish to crates.io, **docs.rs automatically generates documentation** for your crate!

- Your docs will be at: `https://docs.rs/phonon`
- No configuration needed - it happens automatically
- It uses `cargo doc` to build your docs

### Custom Documentation

You can control how docs.rs builds your documentation:

```toml
# In Cargo.toml
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

---

## Current Status for Phonon

### ✅ Already Complete

- [x] README.md (comprehensive)
- [x] Description in Cargo.toml
- [x] Authors in Cargo.toml
- [x] Module documentation exists (63 files with `//!`)
- [x] Extensive docs/ directory

### ❌ Needed Before Publishing

- [ ] Create LICENSE file (MIT, as stated in README)
- [ ] Add `license = "MIT"` to Cargo.toml
- [ ] Add `repository` field to Cargo.toml
- [ ] Add `keywords` to Cargo.toml
- [ ] Add `categories` to Cargo.toml
- [ ] Add `readme = "README.md"` to Cargo.toml
- [ ] Add code examples to key module documentation
- [ ] Run `cargo publish --dry-run` to verify
- [ ] Create crates.io account and login
- [ ] Decide on version number for first release

### 📝 Optional But Recommended

- [ ] Add more inline code examples in `///` doc comments
- [ ] Create CHANGELOG.md
- [ ] Set up GitHub Actions for CI
- [ ] Add badges to README (build status, crates.io version, docs.rs)

---

## Quick Reference

```bash
# Documentation
cargo doc --no-deps --open          # Generate and view docs
cargo test --doc                    # Test code examples in docs

# Publishing
cargo login <token>                 # One-time setup
cargo publish --dry-run            # Check package before publishing
cargo package                      # Create .crate file to inspect
cargo publish                      # Actually publish to crates.io

# Version management
# Edit Cargo.toml to bump version, then:
git tag v0.1.1
git push --tags
cargo publish
```

---

## Resources

- **Cargo Book - Publishing**: https://doc.rust-lang.org/cargo/reference/publishing.html
- **Rust Doc Book**: https://doc.rust-lang.org/rustdoc/
- **Crates.io**: https://crates.io/
- **Docs.rs**: https://docs.rs/
- **SemVer**: https://semver.org/
- **Rust API Guidelines**: https://rust-lang.github.io/api-guidelines/

---

## Example: Publishing Workflow

```bash
# 1. Add required metadata
vim Cargo.toml  # Add license, repository, keywords, categories

# 2. Create LICENSE file
cat > LICENSE << 'EOF'
MIT License
...
EOF

# 3. Verify everything works
cargo build --release
cargo test
cargo doc --no-deps

# 4. Check package contents
cargo publish --dry-run

# 5. Create git tag
git add Cargo.toml LICENSE
git commit -m "Prepare for v0.1.0 release"
git tag v0.1.0
git push origin main --tags

# 6. Publish to crates.io
cargo publish

# 7. Verify on crates.io
# Visit https://crates.io/crates/phonon
# Wait ~5 minutes for docs.rs to build
# Visit https://docs.rs/phonon
```

Done! Your crate is now available for everyone to use:

```bash
cargo install phonon
```

Or in their Cargo.toml:

```toml
[dependencies]
phonon = "0.1.0"
```
