RS-Collector quick ref for agents

Build/lint/typecheck
- Build: cargo build [--release] [--all-features]
- Feature flags: memory_collection, embed_config, yara (use --features a,b)
- Typecheck: cargo check [--all-features]; Docs: cargo doc --no-deps
- Clippy: cargo clippy --all-targets --all-features -D warnings
- Format check: cargo fmt --all -- --check (format with cargo fmt --all)

Tests
- All: cargo test --verbose
- All features: cargo test --all-features --verbose
- Single test (substring): cargo test <pattern>
- Single integration file: cargo test --test <file_stem>
- Single test in file: cargo test --test <file_stem> <test_name>
- Doctests: cargo test --doc

Style guidelines
- Rust 2021; rustfmt defaults; small focused functions
- Imports: group std, external, crate::*; avoid glob; keep sorted
- Errors: anyhow::Result + anyhow::Context; use ?; avoid unwrap/expect/panic in lib code
- Logging: use log macros (error!/warn!/info!/debug!/trace!), not println!; never log secrets
- Types: prefer &str, slices, iterators; avoid unnecessary clones; use Option/Result idiomatically
- Concurrency: tokio for async I/O; rayon for CPU-bound; avoid blocking in async
- Naming: snake_case fns/vars, CamelCase types/traits, SCREAMING_SNAKE_CASE consts; modules lowercase
- Security: validate paths (security::path_validator), scrub creds (security::credential_scrubber); isolate/document unsafe

Assistant rules
- No Cursor or Copilot rules found; if .cursor/rules or .github/copilot-instructions.md appear, mirror constraints here
- Use local .crush/ for scratch artifacts (gitignored)