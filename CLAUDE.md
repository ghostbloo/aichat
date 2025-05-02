# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands
- Build: `cargo build`
- Release build: `cargo build --release`
- Client build: `cd client && pnpm build` (or `npm run build`)

## Test Commands
- Run all tests: `cargo test --all`
- Run specific test: `cargo test test_name`
- Run tests in a module: `cargo test module::path`

## Lint Commands
- Format check: `cargo fmt --all --check`
- Format fix: `cargo fmt --all`
- Lint check: `cargo clippy --all --all-targets -- -D warnings`

## Code Style Guidelines
- Use Rust 2021 edition features
- Follow standard Rust naming conventions (snake_case for variables/functions, CamelCase for types)
- Keep functions focused and reasonably sized
- Add error context with `anyhow` when appropriate
- Use async/await for asynchronous code with Tokio runtime
- Prefer using `?` operator for error propagation
- Format code with rustfmt
- Fix all clippy warnings