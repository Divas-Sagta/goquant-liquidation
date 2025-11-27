# GoQuant Liquidation Engine

This repository implements a prototype liquidation engine for a high-leverage perpetual futures exchange on Solana.

It consists of:

- **Anchor program** (`programs/liquidation_engine`):  
  Smart contract that validates liquidation conditions and executes partial / full liquidations, including insurance fund handling.

- **Rust backend service** (`backend`):  
  Off-chain liquidation engine that monitors positions, enqueues liquidations, and exposes monitoring APIs.

- **Database schema** (`db/migrations`):  
  PostgreSQL tables for liquidation history, bad debt, liquidator stats, insurance fund transactions, and failed liquidations.

- **Documentation** (`docs`):  
  Architecture, liquidation mechanics, APIs, deployment, and testing.

---

## Project Structure

```text
goquant-liquidation/
├── Anchor.toml
├── Cargo.toml
├── programs/
│   └── liquidation_engine/      # On-chain Solana program (Anchor)
├── backend/                     # Off-chain liquidation engine (Rust, Axum, Tokio)
├── db/
│   └── migrations/              # PostgreSQL schema
└── docs/                        # Technical documentation
````

---

## Building & Running

### 1. Smart Contract (Anchor Program)

```bash
cd ~/goquant-liquidation

# Build program for SBF
anchor build --arch sbf
```

This compiles the `liquidation_engine` program.

### 2. Backend Liquidation Engine

```bash
cd ~/goquant-liquidation/backend

# Build with latest Rust
cargo +stable build

# Run engine + HTTP API
cargo +stable run
```

The backend:

* Runs a periodic liquidation check loop.
* Enqueues undercollateralized positions into a priority queue.
* Logs which positions would be liquidated.
* Exposes:

  * `GET /health`
  * `GET /liquidations/pending`

Example:

```bash
curl http://localhost:8080/health
curl http://localhost:8080/liquidations/pending
```

---

## Testing

### 1. On-chain Program Unit Tests

```bash
cd ~/goquant-liquidation/programs/liquidation_engine
cargo +stable test
```

These tests cover:

* Maintenance margin tiers.
* PnL calculations for long/short.
* Equity clamping.
* Margin ratio calculation.

### 2. Backend

```bash
cd ~/goquant-liquidation/backend
cargo +stable test      # currently no unit tests, but crate builds
```

See `docs/TESTING.md` for more detail.

---

## Documentation

Key docs are under `docs/`:

* `ARCHITECTURE.md` – System components & interactions.
* `LIQUIDATION_MECHANICS.md` – Margin, partial/full liquidation logic, insurance fund.
* `API.md` – HTTP endpoints (`/health`, `/liquidations/pending`, and planned endpoints).
* `DEPLOYMENT.md` – How to run locally and conceptual production deployment.
* `TESTING.md` – How to run tests and what they cover.

