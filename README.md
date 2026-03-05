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
| `cross-contract-reentrancy` | 🔴 Critical | Reentrancy traced across contract boundaries, naming the exact callee |
| `trait-dispatch` | 🔴 Critical | State mutation around calls through `<trait>` parameters — any conforming contract could be passed in |
| `access-control` | 🔴 High | Public functions that modify state without `tx-sender` or `contract-caller` checks |
| `integer-underflow` | 🟠 High | Subtraction on `uint` without a preceding bounds check |
| `unchecked-returns` | 🟡 Medium | External calls whose return values are not wrapped in `try!` or `unwrap!` |
| `integer-overflow` | 🔵 Low | Addition without upper bound validation |

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

### Single file analysis

```bash
# analyze a single contract
clarus check mycontract.clar

# analyze a contract anywhere on your system
clarus check /path/to/contract.clar

# output as JSON
clarus check mycontract.clar --json

# exit with code 1 if issues found (for CI/CD)
clarus check mycontract.clar --strict
```

### Multi-contract project analysis

```bash
# analyze all .clar files in a directory
clarus scan ./contracts

# with JSON output
clarus scan ./contracts --json

# strict mode for CI/CD gates
clarus scan ./contracts --strict
```

### Call graph visualization

```bash
# show all contract interactions
clarus graph ./contracts

# show only cross-contract calls
clarus graph ./contracts --cross-only
```

---

## Testing External Contracts

Clarus works on any `.clar` file or directory anywhere on your system.

**During development (before installing globally):**

```bash
cargo run -- check ~/Desktop/mycontract.clar
cargo run -- scan ~/projects/my-stacks-app/contracts
cargo run -- graph ~/projects/my-stacks-app/contracts
```

**Using the release binary directly:**

```bash
cargo build --release
./target/release/clarus check ~/Desktop/mycontract.clar
./target/release/clarus scan ~/projects/stacks-defi/contracts
```

**After installing globally:**

```bash
clarus check ~/Desktop/mycontract.clar
clarus scan ~/projects/stacks-defi/contracts
clarus graph ~/projects/stacks-defi/contracts --cross-only
```

---

## Example Output

### Single contract

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

### Multi-contract scan

```
Clarus — Cross-Contract Analysis
Directory: ./contracts
────────────────────────────────────────────────────────────

  [1] [CRITICAL] Cross-Contract Reentrancy

    Contract :  vault
    Function :  withdraw
    Location :  Line 10
    Issue    :  'map-set' mutated after calling token.transfer on line 9
                re-entry possible before state is updated
    Fix      :  Move 'map-set' to before the contract-call? to token.transfer

────────────────────────────────────────────────────────────

  Result : 12 total findings across 3 contracts
           4 critical
           5 high
           2 medium
           1 low
```

### Call graph

```
Clarus — Call Graph
Directory: ./contracts
────────────────────────────────────────────────────────────

  Contracts :  4
  Functions :  11
  Edges     :  9

────────────────────────────────────────────────────────────
  Call Edges:

  governance.execute-proposal  ──▶  token.transfer  (line 22)
  oracle.update-and-notify     ──▶  governance.notify-price-update  (line 33)
  pool.remove-liquidity        ──▶  token.transfer  (line 17)
  staking.unstake              ──▶  token.transfer  (line 21)
  staking.claim-rewards        ──▶  token.transfer  (line 34)
```

---

## CI/CD Integration

Add Clarus to your GitHub Actions workflow:

```yaml
- name: Run Clarus
  run: |
    cargo install --path .
    clarus scan contracts/ --strict
```

The `--strict` flag causes Clarus to exit with code `1` if any findings are detected, blocking the pull request.

For single file checks:

```yaml
- name: Run Clarus
  run: |
    cargo install --path .
    clarus check contracts/vault.clar --strict
```

---

## Vulnerability Patterns

### Reentrancy

When a contract performs an external call before updating its own state, a malicious contract can re-enter and exploit the stale state.

**Vulnerable:**
```clarity
(define-public (withdraw (amount uint))
  (begin
    (stx-transfer? amount (as-contract tx-sender) tx-sender)
    (map-set balances tx-sender u0)
  ))
```

**Safe (checks-effects-interactions):**
```clarity
(define-public (withdraw (amount uint))
  (begin
    (map-set balances tx-sender u0)
    (try! (stx-transfer? amount (as-contract tx-sender) tx-sender))
    (ok true)
  ))
```

### Cross-Contract Reentrancy

Only detectable with multi-contract analysis. When contract A calls contract B and then updates state, contract B can call back into contract A before the state update completes.

```
vault.withdraw ──▶ token.transfer ──▶ vault.withdraw (re-entry!)
```

Clarus traces this path across contracts and names the exact callee involved.

### Trait-Based Dispatch

When a function accepts a `<trait>` parameter, any conforming contract can be passed in at runtime — including a malicious one.

**Vulnerable:**
```clarity
(define-public (swap (token <ft-trait>) (amount uint))
  (begin
    (contract-call? token transfer amount tx-sender recipient)
    (map-set reserves recipient (+ reserve amount))
  ))
```

### Unchecked Return Values

Calling `stx-transfer?` or `contract-call?` without `try!` means failures are silently ignored.

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
│   ├── main.rs                    — CLI entry point
│   ├── ast.rs                     — AST node definitions
│   ├── parser.rs                  — Clarity S-expression parser
│   ├── analyzer.rs                — Orchestrates all detectors
│   ├── reporter.rs                — Terminal and JSON output
│   ├── loader.rs                  — Multi-file contract loader
│   ├── registry.rs                — Contract registry for project analysis
│   ├── callgraph.rs               — Call graph builder and cycle detector
│   └── detector/
│       ├── mod.rs
│       ├── reentrancy.rs          — Single-contract reentrancy
│       ├── cross_contract.rs      — Cross-contract reentrancy
│       ├── trait_dispatch.rs      — Trait-based dispatch detection
│       ├── integer_overflow.rs    — Integer underflow and overflow
│       ├── access_control.rs      — Missing access control
│       └── unchecked_returns.rs   — Unchecked return values
└── tests/
    ├── contracts/
    │   ├── vulnerable.clar
    │   ├── safe.clar
    │   └── bug.clar
    └── multicontract/
    |   ├── vault.clar
    |   ├── token.clar
    |   ├── rewards.clar
    |   ├── dex.clar
    |__ multicontract2/
        ├── governance.clar
        ├── oracle.clar
        ├── pool.clar
        ├── staking.clar
```

---

## Roadmap

### Phase 1 — MVP ✅
- Single file analysis
- Reentrancy detection
- Integer underflow and overflow
- Missing access control
- Unchecked return values
- CLI with JSON output and strict mode

### Phase 2 ✅
- Multi-file and multi-contract analysis
- Cross-contract reentrancy detection
- Trait-based dynamic dispatch detection
- Call graph visualization with cycle detection
- Project-wide severity-sorted reports

### Phase 3
- Clarinet plugin integration
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