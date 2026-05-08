# API Contracts

This document describes the response envelope, error format, pagination, caching, and rate limiting behaviour shared across all Almanac API endpoints.

Interactive API reference is available at `/swagger-ui` and `/scalar` when the server is running.

---

## Base URL

```
http://localhost:8080/api/v1
```

---

## Response Envelope

Every response — success or error — is wrapped in the same envelope:

```json
{
  "success": true,
  "message": "OK",
  "data": { ... },
  "meta": {
    "request_id": "req-1a2b3c-1",
    "execution_time_seconds": 0.0012,
    "timestamp": "2026-05-07T10:00:00Z"
  },
  "error": null
}
```

| Field | Type | Description |
|---|---|---|
| `success` | `boolean` | `true` on success, `false` on error |
| `message` | `string` | `"OK"` on success; human-readable error message on failure |
| `data` | `object \| array \| null` | Payload on success; `null` on error |
| `meta` | `object` | Request metadata (see below) |
| `error` | `object \| null` | Error details on failure; `null` on success |

### Meta object

| Field | Type | Always present | Description |
|---|---|---|---|
| `timestamp` | `string` | Yes | ISO 8601 UTC timestamp of the response |
| `request_id` | `string` | When available | Echoes the `X-Request-Id` header; generated if not provided |
| `execution_time_seconds` | `number` | When available | Wall-clock time from request receipt to response |
| `limit` | `integer` | Paginated endpoints only | Page size used |
| `offset` | `integer` | Paginated endpoints only | Number of items skipped |
| `total_data` | `integer` | Paginated endpoints only | Total matching items across all pages |

---

## Error Format

```json
{
  "success": false,
  "message": "model not found",
  "data": null,
  "meta": {
    "timestamp": "2026-05-07T10:00:00Z"
  },
  "error": {
    "code": "MODEL_NOT_FOUND",
    "details": {
      "provider": "openai",
      "id": "gpt-99"
    }
  }
}
```

### Error codes

| HTTP status | Code | Description |
|---|---|---|
| 400 | `BAD_REQUEST` | Malformed query parameters or request body |
| 404 | `NOT_FOUND` | Generic not-found |
| 404 | `MODEL_NOT_FOUND` | No model matches the given provider + id |
| 404 | `PROVIDER_NOT_FOUND` | No provider matches the given id |
| 404 | `ALIAS_NOT_FOUND` | No alias matches the given key |
| 408 | `REQUEST_TIMEOUT` | Request exceeded the 10-second server-side timeout |
| 413 | `PAYLOAD_TOO_LARGE` | Request body exceeds 64 KB |
| 429 | `RATE_LIMIT_EXCEEDED` | Per-IP request rate exceeded (when `RATE_LIMIT_RPS` is set) |
| 500 | `INTERNAL_SERVER_ERROR` | Unexpected server error |

---

## Pagination

Endpoints that return lists (`GET /models`, `GET /search`) support offset pagination via query parameters:

| Parameter | Default | Description |
|---|---|---|
| `limit` | `20` | Number of items to return |
| `offset` | `0` | Number of items to skip |

The `meta` object on paginated responses includes `limit`, `offset`, and `total_data` so clients can determine if more pages exist:

```
has_more = offset + limit < total_data
next_offset = offset + limit
```

---

## Caching

Catalog endpoints (`/models`, `/providers`, `/aliases`, `/catalog/*`) support HTTP conditional requests:

- Responses include `ETag` and `Cache-Control: public, max-age=300` headers.
- Send `If-None-Match: <etag>` to get `304 Not Modified` when the catalog has not changed.
- The ETag changes whenever the catalog is reloaded (on startup or SIGHUP).

---

## Rate Limiting

Rate limiting is **disabled by default**. When enabled via the `RATE_LIMIT_RPS` environment variable, the following headers are included in every response:

| Header | Description |
|---|---|
| `X-RateLimit-Remaining` | Requests remaining in the current 1-second window |
| `Retry-After` | Seconds to wait before retrying (only on `429` responses) |

---

## Request ID

Pass `X-Request-Id` with any request to correlate logs with your own trace IDs. The server echoes it back in the response header and in `meta.request_id`. If omitted, the server generates one automatically.

---

## Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/v1/health` | Server health and version |
| `GET` | `/api/v1/providers` | List all providers |
| `GET` | `/api/v1/providers/{id}` | Single provider detail |
| `GET` | `/api/v1/models` | List models with filtering, sorting, pagination |
| `GET` | `/api/v1/models/{provider}/{id}` | Single model detail |
| `GET` | `/api/v1/aliases` | List all aliases |
| `GET` | `/api/v1/aliases/{alias}` | Resolve a single alias |
| `POST` | `/api/v1/validate` | Validate a model string |
| `GET` | `/api/v1/search` | Full-text search across the catalog |
| `GET` | `/api/v1/suggest` | Fuzzy-match suggestions for a model string |
| `GET` | `/api/v1/compare` | Side-by-side comparison of two models |
| `GET` | `/api/v1/catalog/health` | Catalog freshness and coverage stats |
| `GET` | `/api/v1/catalog/issues` | Catalog data quality issues |

Full request/response schemas for each endpoint are available in the interactive docs at `/swagger-ui` or `/scalar`.
