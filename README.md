# Nasiko P1 Tool-Compact Prototype

This bundle contains the prototype for **P1 — Compact Tool Schemas Without Breaking Tool Calls**.

## Contents

- `tool-compact/`: standalone `nasiko-tool-compact` Rust crate.
- `llm-router/examples/compact_tools_eval.rs`: offline evaluator.
- `Cargo.toml`, `Cargo.lock`, and `llm-router/Cargo.toml`: workspace/package changes required by the prototype.

## Install into a Nasiko checkout

Copy the following paths into the root of your Nasiko repository:

```sh
cp -R tool-compact /path/to/nasiko/
cp llm-router/examples/compact_tools_eval.rs /path/to/nasiko/llm-router/examples/
cp Cargo.toml Cargo.lock /path/to/nasiko/
cp llm-router/Cargo.toml /path/to/nasiko/llm-router/
```

If you have local changes to the manifests, manually apply these additions instead of overwriting them:

```toml
# Root Cargo.toml workspace members
"tool-compact",

# Root Cargo.toml workspace dependencies
nasiko-tool-compact = { path = "tool-compact" }

# llm-router/Cargo.toml dependencies
nasiko-tool-compact.workspace = true
```

## Requirements

- Rust/Cargo with Rust 2024 edition support.
- Network access on the first Cargo build to download dependencies.

## Run tests

```sh
cargo test -p nasiko-tool-compact
cargo check -p nasiko-llm-router --example compact_tools_eval
```

## Run the public evaluation set

```sh
curl -fsSL https://registry.nasiko.dev/r/nasiko/compact-tools-eval \
  -o /tmp/compact-tools-eval.json

EVAL_SET=/tmp/compact-tools-eval.json \
OUT=/tmp/compact-tools-out.jsonl \
cargo run --release -p nasiko-llm-router --example compact_tools_eval
```

The evaluator writes one JSONL record per regular case and decoder case. It supports case-specific tool subsets, multiple calls, messages in the compact request, arbitrary stream chunk boundaries, and normalized unknown-tool/invalid-argument errors.

## Current prototype scope

Implemented: compact tool signatures, JSON argument decoding, required/optional fields, primitive types, arrays, nested objects, enums, formats, fail-closed validation, streaming chunk-boundary handling, and the offline evaluator.

Not yet included: live model/provider calls, `o200k_base` token counting, opt-in router request/response transformation, or true incremental emission before `StreamDecoder::finish()`.
