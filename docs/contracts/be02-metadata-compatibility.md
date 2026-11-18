# BE-02 metadata compatibility contract

Base: `2d4278721b8899a26971a8db3a78cb37e972fb01`

## Writer behavior

Generated records are discovery candidates. Missing context and output limits serialize as `null` with a matching `*_state: unknown`; scraped zero or nonzero price candidates stay unpublished because `ScrapedModel` does not identify a unit, currency, seller or price evidence. Pricing serializes as `null` and `pricing_state: unknown`. Lifecycle and endpoint family are `unknown`. Modality and parameter lists are not guessed. Capability facts are an empty map with `capabilities_state: unknown`. Confidence is `unknown`.

The CLI's retrieval date is not a verified date. Generated records omit source `last_verified` and leave model `last_verified` null. YAML is serialized through the YAML library, and provider/model IDs are checked as single catalog path components before writing.

## Schema and API behavior

Existing model YAML remains valid without migration. Existing records retain numeric context/output values, modality arrays, status, pricing, dates and capability flags. New state fields are optional for legacy inputs. The API DTO accepts these old documents and serializes their known values as before. New candidate records return null values (and explicit state fields) instead of numeric/date defaults. The status enum adds `unknown`; existing lifecycle values are unchanged. Source verification dates are optional.

The OpenAPI model types derive from the Rust DTOs; nullable context/output/modality/pricing and optional state fields therefore appear in generated component schemas. Consumers must handle null and unknown. The field names and versioned path remain unchanged; all prior catalog records remain loadable.

Validation treats unknown lifecycle as a warning, and unknown parameter or modality evidence as a warning rather than falsely claiming support or rejection. Filters requiring known context or modality exclude candidates whose values are unknown.

## Deliberate boundaries

This contract does not normalize prices across units, currencies, regions, hosting routes or qualifiers. It does not reclassify historical zero-price entries or bulk-edit model YAML. BE-10 must define the typed price contract and reconcile PR #49's `unpriced_reason` before pricing implementation. BE-03 may only write candidates through this contract and must retain failed/partial receipts without retirement implications.
