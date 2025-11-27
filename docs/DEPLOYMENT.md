# Deployment & Operations Guide

This document explains how to run the liquidation engine locally and outlines how it would be deployed in a production environment.

---

## 1. Prerequisites

- Rust (stable) for backend.
- Rust `1.75` + Anchor `0.29` + Solana CLI for on-chain program.
- PostgreSQL instance.
- Redis (optional, for high-performance position cache).
- Node.js (for Anchor tests / TS tooling, if needed).

---

## 2. Local Development – Smart Contract

From repo root:

```bash
cd ~/goquant-liquidation

# build Anchor program for SBF
anchor build --arch sbf
````

This compiles the `liquidation_engine` program into an SBF shared object ready for deployment on a local validator or devnet.

---

## 3. Local Development – Backend

From backend folder:

```bash
cd ~/goquant-liquidation/backend

# build with latest stable Rust
cargo +stable build

# run the liquidation engine + HTTP API
cargo +stable run
```

Default behavior:

* Starts `LiquidationEngine` loop (currently using a dummy position for demonstration).
* Exposes HTTP API:

  * `GET /health`
  * `GET /liquidations/pending`

---

## 4. Database Setup

Apply the initial migrations:

```bash
psql <connection-string> -f db/migrations/0001_init.sql
```

This creates tables for:

* `liquidation_events`
* `bad_debt_events`
* `liquidator_stats`
* `insurance_fund_transactions`
* `failed_liquidations`
