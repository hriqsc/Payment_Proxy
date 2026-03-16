# Payment Controller

A lightweight payment proxy built in Rust. Receives payment requests, routes them to the appropriate processor, and exposes a summary endpoint for auditing.

Built for the [Rinha de Backend 2025](https://github.com/zanfranceschi/rinha-de-backend-2025/tree/main) challenge.

---

## How it works

Incoming requests are placed into an in-memory channel and immediately acknowledged with `202 Accepted`. A pool of async workers consumes the channel and handles delivery:

1. Try the **default processor** up to 3 times (non-blocking — each failure requeues the job and the worker moves on immediately)
2. If the default is exhausted or down, try the **fallback processor** once
3. If both fail, persist the payment to Redis for retry

Two background tasks run independently:

- **Health checker** — polls both processors every 5 seconds and updates their status in Redis
- **Queue drainer** — periodically reads unprocessed payments from Redis and requeues them into the worker channel

---

## Architecture

```
POST /payments
     │
     ▼
 [HTTP Handler] ──try_send──► [mpsc channel] ──► [Worker 1]
                                                  [Worker 2]  ──► default (up to 3x, non-blocking)
                                                  [Worker N]  ──► fallback (1x)
                                                                ──► Redis queue (if both fail)
                                                                        │
                                              [Queue Drainer] ◄─────────┘
                                              [Health Checker] ──► Redis (processor status)
```

---

## Configuration

All configuration is done via environment variables:

| Variable | Description |
|---|---|
| `BACKEND_ADDRESS` | Address the proxy listens on (e.g. `0.0.0.0:8080`) |
| `DF_ADDRESS` | Base URL of the default payment processor |
| `FB_ADDRESS` | Base URL of the fallback payment processor |
| `REDIS_ADDRESS` | Redis connection string (e.g. `redis://localhost:6379`) |

---

## Tuning

The following constants in `main.rs` can be adjusted for your load profile:

| Constant | Default | Description |
|---|---|---|
| `WORKER_COUNT` | `10` | Number of concurrent payment workers |
| `CHANNEL_BUFFER` | `10_000` | Max queued payments before returning `503` |
| `HEALTH_CHECK_INTERVAL_SECS` | `5` | How often health checks and queue draining run |
| `REQUEST_TIMEOUT_SECS` | `3` | HTTP timeout per processor request |

---

## API

Full endpoint specification: [INSTRUCOES.md](https://github.com/zanfranceschi/rinha-de-backend-2025/blob/main/INSTRUCOES.md#detalhes-dos-endpoints)

### `POST /payments`

Accepts a payment request. Returns `202 Accepted` immediately — processing happens asynchronously.

```json
{
  "correlationId": "4a7901b8-7d26-4d9d-aa19-4dc1c7cf60b3",
  "amount": 19.90
}
```

Returns `503 Service Unavailable` if the internal channel is full.

### `GET /payments-summary`

Returns aggregated statistics for both processors within a time range.

| Query param | Required | Format | Description |
|---|---|---|---|
| `to` | Yes | RFC3339 | End of the time range |
| `from` | No | RFC3339 | Start of the time range (defaults to all time) |

```json
{
  "default": {
    "totalRequests": 43236,
    "totalAmount": 415542345.98
  },
  "fallback": {
    "totalRequests": 423545,
    "totalAmount": 329347.34
  }
}
```

---

## Running the tests

Instructions for the official load test suite: [rinha-test/README.md](https://github.com/zanfranceschi/rinha-de-backend-2025/blob/main/rinha-test/README.md)