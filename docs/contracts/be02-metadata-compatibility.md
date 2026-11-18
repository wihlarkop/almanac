# BE-02 metadata compatibility contract

Base: `2d4278721b8899a26971a8db3a78cb37e972fb01`

## Writer behavior

Generated records are discovery candidates. Missing context and output limits serialize as `null` with a matching `*_state: unknown`; scraped zero or nonzero price candidates stay unpublished because `ScrapedModel` does not identify a unit, currency, seller or price evidence. Pricing serializes as `null` and `pricing_state: unknown`. Lifecycle and endpoint family are `unknown`. Modality and parameter lists are not guessed. Capability facts are an empty map with `capabilities_state: unknown`. Confidence is `unknown`.

The CLI's retrieval date is not a verified date. Generated records omit source `last_verified` and leave model `last_verified` null. YAML is serialized through the YAML library, and provider/model IDs are checked as single catalog path components before writing.

## Schema and API behavior

Existing model YAML remains valid without migration. Existing records retain numeric context/output values, modality arrays, status, pricing, dates and capability flags. New state fields are optional for legacy inputs. The API DTO accepts these old documents and serializes their known values as before. Candidate YAML may store explicit nulls for unknown values, while API JSON omits unknown value fields and returns the corresponding `*_state: unknown`; consumers therefore keep the old omission behavior and can opt into the state fields. The status enum adds `unknown`; existing lifecycle values are unchanged. Source verification dates are optional.

The OpenAPI model types derive from the Rust DTOs; context/output/modality/pricing and verification fields are optional and non-null when present, and the new state fields are optional enums. Consumers must handle omitted values and `unknown`. The field names and versioned path remain unchanged; all prior catalog records remain loadable.

Validation treats unknown lifecycle as a warning, and unknown parameter or modality evidence as a warning rather than falsely claiming support or rejection. Not-applicable parameter or modality evidence has a distinct warning. Capability filters require known capability evidence. Pricing availability, free/paid filters and token-price ranking distinguish known free from unknown or not-published pricing, include request fees when deciding whether a record is free, and rank only comparable text-token prices in a common currency. Cross-currency or non-token comparisons are withheld.

## Deliberate boundaries

This contract does not normalize prices across units, currencies, regions, hosting routes or qualifiers. It does not bulk-edit model YAML. Legacy all-zero text-token prices retain their previous free interpretation; non-text zero prices remain unknown unless explicitly classified. BE-10 must define the typed price contract and reconcile PR #49's `unpriced_reason` before pricing implementation. PR #34 must update its TypeScript status union to accept `unknown` and consume the state fields; it remains dependent on backend PR #49 for confidence statistics. BE-03 may only write candidates through this contract and must retain failed/partial receipts without retirement implications.
