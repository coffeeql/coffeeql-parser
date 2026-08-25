# coffeeql-parser

The parser for CoffeeQL.

Converts a token stream from `coffeeql-lexer` into a
typed AST — a `QueryNode` that the CoffeeQL planner
uses to build an execution plan.

## What this does

\`\`\`
[Collection("users"), Dot, Where, ...]
         ↓
QueryNode {
  collection: "users",
  kind: Structured,
  chain: [
    ChainOp::Where(Expression::Binary {
      left: Field("plan"), op: Eq, right: Text("pro")
    }),
    ChainOp::Cup(10)
  ]
}
\`\`\`

## Crate structure

- `src/ast.rs`       — QueryNode, ChainOp, Expression types
- `src/mod.rs`       — Recursive descent parser
- `src/chain.rs`     — ChainState validation (order enforcement)
- `src/validator.rs` — Semantic validation
- `src/visitor.rs`   — AST visitor trait
- `src/pratt.rs`     — Pratt parser for expressions
- `src/recovery.rs`  — Error recovery
- `src/tests.rs`     — Test suite

## Usage

\`\`\`rust
use coffeeql_parser::Parser;
use coffeeql_lexer::Lexer;

let tokens = Lexer::new("users[].where(plan = \"pro\").cup(10)").tokenize();
let ast = Parser::new(tokens).parse();
\`\`\`

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Part of CoffeeQL

- npm: `npm install coffeeql`
- pip: `pip install coffeeql`
- docs: [coffeeql.dev](https://coffeeql.dev)

## License

Apache 2.0 + Commons Clause
