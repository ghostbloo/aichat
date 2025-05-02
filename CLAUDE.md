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

## Codebase info
### Config
The Config struct (defined in `src/config/mod.rs`) is a comprehensive configuration manager for the aichat application. It's responsible for:

1. Managing AI model settings (model_id, temperature, top_p)
2. Handling app behavior configuration (dry_run, stream, save, etc.)
3. Managing file paths for various components (roles, sessions, rags, functions)
4. Supporting state management (working_mode, role, session, agent)
5. Providing access to environment variables and configuration files
6. Initializing and configuring the AI client and its functions
7. Managing document loaders and RAG functionality
8. Providing UI/UX settings (prompt formatting, highlighting, themes)

The Config object is the central hub that coordinates all aspects of the application's configuration and state.

### Sessions
The Session struct (defined in `src/config/session.rs`) manages conversational state and history with AI models. Key responsibilities include:

1. Storing and managing message history with the AI model
2. Handling persistence of conversations to disk
3. Managing context including system prompts, user inputs, and AI responses
4. Tracking token usage and compression of conversation history
5. Supporting various session features:
   - Loading/saving sessions to files
   - Auto-naming of temporary sessions
   - Managing role assignments within sessions
   - Synchronizing with agent configurations
   - Compressing message history to stay within token limits

Sessions maintain their state (dirty flag, path) and provide methods for building message context for each API call to the AI model.

### REPL and CLI
The REPL (Read-Eval-Print Loop) and CLI components (defined in `src/repl/mod.rs` and `src/cli.rs`) provide interactive interfaces for users. Key features include:

1. Command handling for 35+ built-in commands (like `.help`, `.role`, `.session`, `.agent`)
2. Line editing with configurable keybindings (emacs/vi modes)
3. Command completion and syntax highlighting
4. Interactive prompts for configuration selection
5. Support for multiline input with `:::` delimiters
6. Streaming output from AI models
7. Interactive abort handling for long-running operations
8. Managing state transitions between roles, sessions, and agents
9. Command validation based on current state

The REPL maintains its editor state and handles input/output processing, while delegating AI interaction to the appropriate client handlers.

### Web UI
The Web UI component (defined in `src/web/mod.rs` and `src/serve.rs`) provides HTTP-based access to aichat functionality. Key features include:

1. Embedded HTML assets for web interfaces:
   - Playground UI for interactive chat
   - Arena UI for comparing responses from different models
2. RESTful API endpoints for accessing:
   - Models (listing available AI models)
   - Roles (browsing and retrieving role configurations)
   - Sessions (accessing conversational history)
   - Agents (retrieving agent definitions and configurations)
   - RAGs (retrieval-augmented generation setups)
3. JSON response formatting for all API endpoints
4. Server configuration with customizable address binding
5. API-based access to the same functionality available in the CLI

The Web UI serves as an alternative interface to the REPL, allowing for web-based interactions with the AI models and programmatic access to configuration.
