# Contributing to Kojacoord Proxy

Thank you for your interest in contributing to Kojacoord Proxy! This document provides guidelines and instructions for contributors.

## Code of Conduct

By participating in this project, you agree to uphold our Code of Conduct (see [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)).

## How to Contribute

### Reporting Bugs

Before creating bug reports, please check the existing issues to avoid duplicates. When creating a bug report, include:

- A clear and descriptive title
- Steps to reproduce the issue
- Expected behavior vs. actual behavior
- Environment details (proxy version, Rust version, OS, Minecraft versions)
- Relevant logs or error messages
- Configuration files (with sensitive information redacted)

### Suggesting Enhancements

Enhancement suggestions are welcome. Please:

- Use a clear and descriptive title
- Provide a detailed description of the proposed enhancement
- Explain why this enhancement would be useful
- Provide examples or mockups if applicable

### Pull Requests

#### Before Submitting

1. **Fork the repository** and create your branch from `main`
2. **Make your changes** following the coding standards below
3. **Write tests** for new functionality
4. **Update documentation** as needed
5. **Ensure all tests pass**: `cargo test`
6. **Format your code**: `cargo fmt`
7. **Run linter**: `cargo clippy`

#### PR Guidelines

- Keep PRs focused on a single issue or feature
- Write clear commit messages
- Reference related issues in your PR description
- Include tests for new features
- Update relevant documentation
- Ensure CI checks pass before requesting review

#### Commit Message Format

Follow conventional commits:

```
feat: add new protocol version support
fix: resolve memory leak in connection pool
docs: update API documentation
refactor: simplify packet parsing logic
test: add integration tests for auth module
```

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Cargo (included with Rust)
- Git

### Building

```bash
# Clone your fork
git clone https://github.com/yourusername/kojacoord-proxy.git
cd kojacoord-proxy

# Build in debug mode
cargo build

# Build in release mode
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in release mode
cargo test --release
```

### Code Style

- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Follow Rust naming conventions
- Add documentation comments (`///`) for public APIs
- Keep functions focused and reasonably sized

## Coding Standards

### Rust Guidelines

- Use `Result<T, E>` for error handling, avoid `panic!` in production code
- Prefer `Arc<Mutex<T>>` or `Arc<RwLock<T>>` for shared state
- Use `async/await` for I/O operations
- Document public APIs with rustdoc comments
- Write unit tests for non-trivial functions
- Use meaningful variable and function names

### Documentation

- Document all public APIs with `///` comments
- Include examples in documentation where helpful
- Update the README for user-facing changes
- Update the wiki for architectural changes
- Keep comments concise and technical

### Testing

- Write unit tests for new functionality
- Test error conditions and edge cases
- Use descriptive test names
- Keep tests independent and fast
- Mock external dependencies where appropriate

## Project Structure

```
kojacoord-proxy/
├── crates/
│   ├── protocol/          # Protocol definitions and codecs
│   ├── netty/             # Network layer with encryption
│   ├── auth/              # Authentication pipeline
│   ├── proxy-core/        # Core proxy logic
│   ├── anticheat/         # Anti-cheat detection
│   ├── config/            # Configuration management
│   ├── api/               # Public API for plugins
│   └── dashboard-api/     # Dashboard REST API
├── src/                   # Main entry point
├── docs/                  # Documentation
├── .github/               # GitHub configuration
│   └── ISSUE_TEMPLATE/    # Issue templates
├── Cargo.toml             # Workspace configuration
├── Cargo.lock             # Dependency lock file
├── LICENSE                # MIT License
└── README.md              # Project documentation
```

## Getting Help

- Check existing [documentation](docs/Usage.md)
- Search [existing issues](https://github.com/yourusername/kojacoord-proxy/issues)
- Ask questions in a new issue with the `question` label
- Join our community discussions (link to be added)

## License

By contributing to Kojacoord Proxy, you agree that your contributions will be licensed under the MIT License.
