# Clarus

> Static security analyzer for Clarity smart contracts on the Stacks blockchain.

Clarus detects common vulnerability patterns in `.clar` files before you deploy — giving developers fast, actionable feedback without needing a full audit.

---

## Why Clarus?

Clarity was designed to be safe — but safe-by-design does not mean bug-free. As the Stacks ecosystem grows and sBTC brings real Bitcoin liquidity into DeFi, the stakes of a single vulnerability rise dramatically.

Clarus fills a gap: **there is currently no dedicated static analysis tool for Clarity**. Clarus aims to be the Slither of the Stacks ecosystem.

---

## Detectors

| ID | Severity | Description |
|----|----------|-------------|
| `reentrancy` | 🔴 Critical | State mutation after `stx-transfer?`, `contract-call?`, or other external interactions |
| `access-control` | 🔴 High | Public functions that modify state without `tx-sender` or `contract-caller` checks |
| `integer-underflow` | 🟠 High | Subtraction on `uint` without a preceding bounds check |
| `integer-overflow` | 🔵 Low | Addition without upper bound validation |
| `unchecked-returns` | 🟡 Medium | External calls whose return values are not wrapped in `try!` or `unwrap!` |

---

## Installation

### From source

```bash
git clone https://github.com/Nanle-code/clarus.git
cd clarus
cargo build --release
```

The binary will be at `./target/release/clarus`.

Optionally install globally so you can run `clarus` from anywhere on your machine:

```bash
cargo install --path .
```

---

## Usage

```bash
# analyze a contract in the current directory
clarus mycontract.clar

# analyze a contract anywhere on your system
clarus /path/to/contract.clar

# output as JSON (for CI/CD pipelines)
clarus mycontract.clar --json

# exit with code 1 if issues found (for CI/CD gates)
clarus mycontract.clar --strict
```

---

## Testing External Contracts

Clarus works on any `.clar` file anywhere on your system — not just contracts inside the Clarus project folder.

**During development (before installing globally):**

```bash
# analyze any contract using cargo run
cargo run -- ~/Desktop/mycontract.clar
cargo run -- ~/projects/my-stacks-app/contracts/vault.clar

# with flags
cargo run -- ~/Desktop/mycontract.clar --json
cargo run -- ~/Desktop/mycontract.clar --strict
```

**Using the release binary directly:**

```bash
# build the optimized binary first
cargo build --release

# then point it at any contract
./target/release/clarus ~/Desktop/mycontract.clar
./target/release/clarus ~/projects/stacks-defi/contracts/pool.clar
```

**After installing globally:**

```bash
# run clarus from any directory on your machine
clarus ~/Desktop/mycontract.clar
clarus ~/projects/stacks-defi/contracts/pool.clar
clarus ~/projects/stacks-defi/contracts/pool.clar --json
```

> **Note:** If a contract references other contracts via `use-trait` or `impl-trait`, Clarus will still analyze the file correctly. Cross-contract call tracing across multiple files is coming in Phase 2.

---

## Example Output

```
Clarus — Clarity Smart Contract Analyzer
Analyzing: vault.clar
────────────────────────────────────────────────────────────

  [1] [CRITICAL] Reentrancy

    Function :  withdraw
    Location :  Line 8
    Issue    :  'map-set' called after external interaction on line 6
    Fix      :  Move 'map-set' to before the external call on line 6

────────────────────────────────────────────────────────────

  [2] [HIGH] Missing Access Control

    Function :  deposit
    Location :  Line 3
    Issue    :  Public function 'deposit' modifies state without access control check
    Fix      :  Add (asserts! (is-eq tx-sender contract-owner) (err u403)) at the top

────────────────────────────────────────────────────────────

  Result : 2 total findings
           1 critical
           1 high
```

---

## CI/CD Integration

Add Clarus to your GitHub Actions workflow:

```yaml
- name: Run Clarus
  run: |
    cargo install --path .
    clarus contracts/vault.clar --strict
```

The `--strict` flag causes Clarus to exit with code `1` if any findings are detected, which will fail the workflow and block the pull request.

---

## Vulnerability Patterns

### Reentrancy

The most critical pattern. When a contract performs an external call (`stx-transfer?`, `contract-call?`, etc.) before updating its own state, a malicious contract can re-enter and exploit the stale state.

**Vulnerable:**
```clarity
(define-public (withdraw (amount uint))
  (begin
    (stx-transfer? amount (as-contract tx-sender) tx-sender)  ;; external call first
    (map-set balances tx-sender u0)                            ;; state update after — dangerous
  ))
```

**Safe (checks-effects-interactions):**
```clarity
(define-public (withdraw (amount uint))
  (begin
    (map-set balances tx-sender u0)                            ;; state update first
    (try! (stx-transfer? amount (as-contract tx-sender) tx-sender))
    (ok true)
  ))
```

### Unchecked Return Values

Calling `stx-transfer?` or `contract-call?` without `try!` means failures are silently ignored and execution continues.

**Vulnerable:**
```clarity
(stx-transfer? amount (as-contract tx-sender) tx-sender)
(map-delete balances tx-sender)
```

**Safe:**
```clarity
(try! (stx-transfer? amount (as-contract tx-sender) tx-sender))
(map-delete balances tx-sender)
```

---

## Project Structure

```
clarus/
├── src/
│   ├── main.rs              — CLI entry point
│   ├── ast.rs               — AST node definitions
│   ├── parser.rs            — Clarity S-expression parser
│   ├── analyzer.rs          — Orchestrates all detectors
│   ├── reporter.rs          — Terminal and JSON output
│   └── detector/
│       ├── mod.rs
│       ├── reentrancy.rs    — Reentrancy detector
│       ├── integer_overflow.rs
│       ├── access_control.rs
│       └── unchecked_returns.rs
└── tests/
    └── contracts/
        ├── vulnerable.clar
        ├── safe.clar
        └── bug.clar
```

---

## Roadmap

### Phase 1 — MVP ✅
- Single file analysis
- Reentrancy detection
- Integer underflow/overflow
- Access control
- Unchecked return values
- CLI with JSON output and strict mode

### Phase 2
- Multi-file / multi-contract analysis
- Trait-based dynamic dispatch detection
- Call graph visualization
- Clarinet plugin integration

### Phase 3
- GitHub Action
- VS Code extension
- Web-based playground
- Public Clarity vulnerability database

---

## Contributing

Clarus is open source and welcomes contributions. If you find a Clarity vulnerability pattern that Clarus misses, please open an issue with a minimal reproducing contract.

---

## License

MIT