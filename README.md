# logfmt_nostd

A minimal **logfmt**-style parser library written in Rust, designed for `#![no_std]` environments.

## 🚀 Why this crate?

- **no_std compatible**: usable in embedded systems, WASM, or constrained runtimes.
- **Lightweight parser**: separates a human-readable “message” from key–value attributes.
- **Safe and predictable**: enforces limits on key/value lengths and rejects malformed tokens.
- **Test-driven**: comes with tests ensuring correctness and robustness.

## 📦 Features

- Parses logfmt-style strings into:
  - A free-form **message** (first word sequence or `msg=` override).
  - A collection of **attributes** (`key=value`) up to a max of 25 entries.
- Handles quoted strings, escaped tokens, and malformed input gracefully.
- `#![no_std]` crate, relying only on `alloc` and `core`.
- Uses `Cow<str>` to optimize borrowing vs owning message data.

## ✅ Example usage

```rust
use logfmt_nostd::Log;

let input = r#"this is foo=bar duration=10 value="with spaces" message"#;
let log = Log::parse(input).expect("valid logfmt format");

assert_eq!(log.message(), "this is a message");
assert_eq!(
    log.attributes(),
    &[("foo", "bar"), ("duration", "10"), ("value", "\"with spaces\"")]
);
