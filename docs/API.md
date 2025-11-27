# Liquidation Engine – API

This document describes the HTTP monitoring endpoints exposed by the Rust backend.

> Note: Implementation status is indicated per endpoint (implemented / planned).

---

## Base URL

For local development:

```text
http://localhost:8080
````

---

## 1. GET `/health`

**Status:** Implemented

### Description

Simple health check to verify the backend process is running.

### Request

* **Method:** `GET`
* **Path:** `/health`
* **Query parameters:** none

### Response

```json
{
  "status": "ok"
}
```

---

## 2. GET `/liquidations/pending`

**Status:** Implemented (wired to in-memory queue)

### Description

Returns the list of positions currently at risk and queued for liquidation by the off-chain engine.

The backend reads from the in-memory `LiquidationQueue`, which is populated by `LiquidationEngine` every tick when it detects undercollateralized positions.

### Request

* **Method:** `GET`
* **Path:** `/liquidations/pending`
* **Query parameters:** none

### Response (current implementation)

```json
{
  "positions": [
    "pos-1",
    "pos-1",
    "... repeated IDs as queue grows ..."
  ]
}
```

### Notes

* Each entry is currently a **position ID**.
* In a production-ready version, this would likely be:

  * De-duplicated.
  * Enriched with symbol, margin ratio, and user.
