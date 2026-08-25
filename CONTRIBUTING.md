# Contributing to coffeeql-parser

## What you can work on

Issues labelled `parser` in this repo.

Scope:
- Error messages for chain order violations
- Pratt parser for expression precedence
- Error recovery — report all errors in one pass
- Nested field path validation
- Reserved keyword protection
- Multi-line query support
- .stream() keyword support
- AST visitor pattern
- Test coverage for all parser errors

## Setup

\`\`\`bash
git clone https://github.com/coffeeql/coffeeql-parser
cd coffeeql-parser
cargo build
cargo test
\`\`\`

## PR rules

- cargo test must pass
- cargo clippy must pass
- cargo fmt must be applied
- One issue per PR
- Reference issue number in PR title
