# Contributing to rs-collector

Thank you for your interest in contributing to rs-collector! This document provides guidelines and instructions for contributing to the project.

## Table of Contents
- [Development Setup](#development-setup)
- [Building and Testing](#building-and-testing)
- [CI/CD Process](#cicd-process)
- [Pull Request Process](#pull-request-process)
- [Code Style Guidelines](#code-style-guidelines)
- [Security Considerations](#security-considerations)

## Development Setup

### Prerequisites
- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Git
- Platform-specific dependencies:
  - **Linux**: `libyara-dev` (for YARA support)
  - **macOS**: `brew install yara`
  - **Windows**: YARA installation is complex; basic features work without it

### Clone and Setup
```bash
git clone https://github.com/BurtTheCoder/rs-collector.git
cd rs-collector

# Install Rust toolchain
rustup update stable
rustup component add rustfmt clippy

# Build the project
cargo build
```

## Building and Testing

### Local Development Commands

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Run tests
cargo test

# Run tests with all features
cargo test --all-features

# Build with specific features
cargo build --features memory_collection
cargo build --features embed_config
cargo build --features yara

# Build release version
cargo build --release

# Generate documentation
cargo doc --open
```

### Feature Flags
- `memory_collection`: Enable memory analysis capabilities
- `embed_config`: Embed configuration file in binary
- `yara`: Enable YARA rule scanning support

## CI/CD Process

Our CI/CD pipeline automatically runs on all pull requests and commits to ensure code quality and compatibility.

### Automated Checks

1. **Continuous Integration** (`ci.yml`)
   - Multi-platform testing (Windows, Linux, macOS)
   - Multiple Rust versions (stable, beta, nightly)
   - Code formatting check (`cargo fmt`)
   - Linting (`cargo clippy`)
   - All tests (`cargo test`)
   - Documentation build
   - Code coverage reporting

2. **Security Scanning** (`security.yml`)
   - Daily vulnerability scans with `cargo-audit`
   - License compliance checking with `cargo-deny`
   - Supply chain security verification
   - SARIF reports uploaded to GitHub Security tab

3. **Feature Testing** (`features.yml`)
   - Tests all feature flag combinations
   - Ensures features work independently and together
   - Platform-specific feature validation

4. **Documentation Checks** (`docs.yml`)
   - Validates documentation builds
   - Checks for broken links
   - Ensures examples compile

### Required Checks Before Merge

All pull requests must pass the following checks:
- ✅ Code formatting (`cargo fmt --check`)
- ✅ No clippy warnings (`cargo clippy -- -D warnings`)
- ✅ All tests pass on all platforms
- ✅ Documentation builds without warnings
- ✅ No security vulnerabilities detected

## Pull Request Process

### Before Creating a PR

1. **Ensure your code follows the style guidelines**
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   ```

2. **Run tests locally**
   ```bash
   cargo test
   cargo test --all-features  # If on Linux/macOS with YARA
   ```

3. **Update documentation**
   - Add doc comments for new public APIs
   - Update README.md if adding new features
   - Include examples in doc comments

### PR Guidelines

1. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Write clear commit messages**
   - Use present tense ("Add feature" not "Added feature")
   - Reference issues when applicable (#123)

3. **Fill out the PR template**
   - Describe what changes you made
   - Explain why the changes are necessary
   - List any breaking changes

4. **Keep PRs focused**
   - One feature or fix per PR
   - Split large changes into smaller PRs when possible

5. **Respond to review feedback**
   - Address all comments
   - Push additional commits (don't force-push during review)

## Code Style Guidelines

### Rust Style
- Follow standard Rust conventions
- Use `rustfmt` for consistent formatting
- Prefer explicit error handling over `unwrap()`
- Use `anyhow::Result` for error propagation
- Add appropriate logging with the `log` crate

### Error Handling Example
```rust
// ✅ Good
let file = File::open(&path)
    .context("Failed to open artifact file")?;

// ❌ Avoid
let file = File::open(&path).unwrap();
```

### Documentation
- Document all public APIs
- Include examples in doc comments
- Use `///` for public items, `//` for implementation details

```rust
/// Collects artifacts from the specified path.
///
/// # Arguments
/// * `path` - The path to collect artifacts from
///
/// # Example
/// ```
/// let artifacts = collect_artifacts("/var/log")?;
/// ```
pub fn collect_artifacts(path: &Path) -> Result<Vec<Artifact>> {
    // Implementation
}
```

## Security Considerations

### Handling Sensitive Data
- Never log sensitive information (passwords, keys, PII)
- Use secure methods for handling authentication
- Follow principle of least privilege

### Dependencies
- All dependencies are scanned for vulnerabilities
- License compliance is enforced
- New dependencies should be justified in PRs

### Platform-Specific Code
- Use conditional compilation for platform features
- Test on all supported platforms when possible
- Document platform-specific behavior

## Release Process

Releases are automated through GitHub Actions:

1. **Version Tagging**
   - Create a version tag: `git tag v1.2.3`
   - Push the tag: `git push origin v1.2.3`

2. **Automated Release**
   - Release workflow triggers automatically
   - Builds artifacts for all platforms
   - Creates GitHub release with checksums
   - Artifacts include:
     - Linux: x86_64, aarch64
     - Windows: x86_64, aarch64
     - macOS: x86_64, aarch64

3. **Release Notes**
   - Update CHANGELOG.md before tagging
   - Include breaking changes, new features, and fixes

## Getting Help

- **Issues**: Report bugs or request features via [GitHub Issues](https://github.com/BurtTheCoder/rs-collector/issues)
- **Discussions**: Ask questions in [GitHub Discussions](https://github.com/BurtTheCoder/rs-collector/discussions)
- **Security**: Report security issues privately to the maintainers

## License

By contributing to rs-collector, you agree that your contributions will be licensed under the MIT License.