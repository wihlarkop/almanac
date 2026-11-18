# Pricing offers and their evidence

Design proposal for [issue #64](https://github.com/wihlarkop/almanac/issues/64). Source baseline: `2d4278721b8899a26971a8db3a78cb37e972fb01`. Examples describe billing rules, not verified catalog entries. Field names below are proposed for an opt-in versioned API representation. The BE-02 schema and API contract must be verified before implementation.

## What a price describes

Attach each offer to an exact creator/model revision, seller and service route. An offer contains the conditions for purchasing that service and one or more charge components. A component describes a single meter, denomination, quantity and evidence. This lets a single image edit charge for reference images and output pixels without inventing token prices. `model.provider` always identifies the creator. A host's own tariff is authoritative for its route only. An open-weight license alone says nothing about the cost of inference.

| Axis | Required meaning |
| --- | --- |
| Identity | Creator, model/version, seller identity, route, seller role (creator or third-party host), operation. A family page requires explicit evidence that terms cover the specified version. |
| Market | Original quoted currency or namespaced issuer credit. Carry selling region, product tier, billing mode, effective start/end if published and any trial, tax or negotiated-price qualifiers. Distinguish worldwide or unconditional terms from unspecified scope. |
| Usage | Meter kind, direction and modality, positive denominator quantity, basis (per call, asset, account-period etc.), and whether the component is additive, included in another component, or an alternative. |
| Rate | Exact nonnegative decimal in the quoted denomination. Use a decimal string on the proposed wire format. Keep denominators such as 1,000 and 1,000,000 explicit. Credit quantities are not ISO currencies. |
| Conditions | Describe prompt boundaries (inclusive/exclusive), volume brackets (graduated versus whole-order), quality/resolution, cache read/write and retention, plan eligibility, included allowances, minimum charges and rounding rules as published. A material qualifier that cannot be represented prevents automated ranking. |
| Evidence | Publisher, stable public URL, location within the source (section/table/row), the assertion and its scope, evidence class, reviewer verification date and separately the observation time. Preserve provider effective dates when supplied. |

### States for a scoped meter

| State | Evidence and consequence |
| --- | --- |
| Known price | Published positive rate with a denomination and denominator, supported by evidence for this model, route and conditions. |
| Known free | Explicit zero or a clear no-charge statement for this exact operation, meter, plan and period. One included component or an introductory allowance does not make the entire model free. Retain allowance bounds and overage as separate terms. |
| Not applicable | Evidence establishes that this meter does not apply to this offer. A documented image-only tariff can make token rates inapplicable for that operation. Model modality alone cannot establish billing terms. No amount. |
| Not publicly priced | Evidence of an applicable commercial offering without a public rate, such as an explicit contact-sales term. Preserve the reason. No amount. |
| Unknown | Evidence is missing, ambiguous, conflicting or inaccessible. An absent field, blocked page or unverified parser result goes here. No amount. |

A quote for transcription can be known while the surcharge for an optional feature is unknown. Keep each state scoped. An empty set of offers means no offers have been established. A conflict retains the source assertions for review and prevents selecting a winner by recency alone. A failed later fetch cannot update verification or erase the last reviewed rate. Mark a superseded or expired quote as historical.

### What to meter

| Operation | Possible published basis | Details needed for comparison |
| --- | --- | --- |
| Text and multimodal token inference | Input, output, reasoning, cached read/write or storage, synchronous/batch rates | Token modality, billing mode, prompt length and cache duration. Preserve a separate request/tool/search charge when it applies. |
| Image generation and editing | Per generated/edited image, input/reference/output megapixel, or image tokens | Operation, resolution, quality, number of input/output assets, rounding and any base/minimum charge. |
| TTS, transcription and other audio | Per input character, audio token, second/minute of source audio or synthesized audio | Voice tier, input versus output, number of channels, whether rounded per file and minimum duration. Preserve provider character-count rules. |
| Video | Per rendered second, input second or specified clip | Length, resolution, audio included, separate reference inputs, minimum billable duration, rounding. A clip rate only applies to the specified clip. |
| Request, search, OCR, 3D generation | Per request/search, page, asset, generation or a scoped number of provider credits | Define what counts and any additional usage fees. A request can contain several items. |
| Typed decision | Per question, request, judgment or other meter **only when documented** | Multiple typed questions can share one request. Model output probability/confidence is unrelated to catalog evidence confidence. |
| Credits | Issuer-specific credits per metered resource, or published money per credit pack | Retain issuer, package, plan, expiry and purchase requirements. An advertised credit bundle is insufficient to infer a cash marginal rate. |

Provider documentation illustrates why these dimensions matter. Google's Gemini page splits free and paid tiers, batch and caching. Black Forest Labs distinguishes per-image offers, output megapixels and input references. AWS Polly uses characters and distinguishes voice tiers, while AWS Transcribe documents duration charges and minimums. Runway lists credits for video durations with resolution, audio, reference inputs and rounding conditions. TypeSafe documents typed questions and multiple questions in one call. That API description by itself provides no billing unit. [Google](https://aistudio.google.com/docs/pricing) · [Black Forest Labs](https://bfl.ai/pricing) · [Polly](https://aws.amazon.com/polly/pricing/) · [Transcribe](https://aws.amazon.com/transcribe/pricing/) · [Runway](https://docs.dev.runwayml.com/guides/pricing/) · [TypeSafe](https://docs.typesafe.ai/primitives).

### Evidence confidence and time

* **Official:** the seller's own public material explicitly establishes the scoped rate or no-price state. Include the exact source row and the date a reviewer checked it. A seller's verified quote can be third-party hosting relative to the model creator.
* **Inferred:** an interpretation requires assumptions, for example attributing a family-level marketing claim to one revision. Preserve the assumptions. Exclude from verified coverage and price rankings.
* **Community:** a named third party reports the claim independently. Retain attribution and exclude it from authoritative offers until confirmed by the seller.
* **Unknown:** the available material cannot support the scoped assertion. Leave verification date absent until a reviewer checks the exact claim. The date a file was created, fetched or validated cannot supply that date.

Store the original amount and units even when exact arithmetic produces a display normalization. Evidence class and freshness are independent. An official but expired offer cannot serve as the current tariff. A generic documentation URL, a provider-domain match and model-wide `confidence: official` are insufficient evidence for a particular price. Record review conflicts and defer resolution to a reviewer.

## Comparisons

Compare only quotes for the same requested operation, billing meter/direction, denominator dimension, currency or credit issuer, and matching material conditions. Identify each seller/route in the result. Let clients explicitly select whether to compare creator or named hosting routes. Select the effective offer for the requested date and market. An unknown price-affecting condition or overlapping, ambiguous tiers leaves the result unavailable with a reason.

Exact rescaling within one dimension is allowed (1,000 to 1,000,000 text input tokens, or seconds to minutes for the same continuous audio meter). Retain per-request minima and rounding. Comparing a workload total requires the requested quantities, all applicable additive charges and complete eligible offers for that workload. An individual component comparison must identify its meter and must never be labeled a total. Ties remain ties. Exclude unknown or inapplicable components and disclose exclusions. If fewer than two eligible offers remain, report insufficient comparable offers. Do not convert currencies, characters to tokens, audio minutes to words or credits to money using estimates.

For example, two transcription prices expressed per second and per minute can be compared as rates after exact rescaling if their terms match. If one rounds up each file, their totals require the file durations. A hosted USD quote and a creator EUR quote retain separate offer identities, even if both meter output images. An image generation fee and TTS character rate have no common unit.

## Current API and compatibility

At the baseline, `schema/model.schema.json:132` makes pricing optional but requires currency, input and output when present. Explicit null falls outside its object schema. `server/src/catalog.rs:32` omits absent pricing on serialization, while `Pricing` starts at line 74 with mandatory numeric token prices. Optional fields include batch, cached, audio, image, page and request rates, tiers and notes. The catalog contains an image **edit** with `per_image` and a six-second video clip price expressed in prose. These are examples of stored shapes, not evidence of current provider rates. [Image edit](../../models/alibaba/qwen-image-edit-max.yaml) · [Video clip](../../models/amazon/nova-reel-v1.yaml).

The current writer substitutes zero for missing token prices (`scraper/src/writer.rs:15`). Global stats count a model as free when both token rates are zero and rank raw numeric rates (`server/src/state.rs:118`). Listing filters and sorts use raw input/output values (`server/src/routes/models.rs:388`, `:478`). Compare ranks raw prices and uses fixed character/audio multipliers for `comparable_cost` (`server/src/routes/compare.rs:234`, `:264`). The catalog coverage routes treat presence of the pricing object as coverage (`server/src/routes/catalog.rs:110`). The validator also counts object presence (`validator/src/main.rs:452`). These paths need one eligibility rule. An old zero, even in a non-chat row, cannot automatically be promoted to free or changed to unknown across the catalog. Review source evidence before classifying it.

1. **Preserve v1 reads and existing YAML.** Consume the independently verified BE-02 representation for absent and inapplicable values including its precise omitted/null behavior. Leave legacy fields available for existing records according to that contract. Preserve source amounts verbatim as legacy data. Do not parse prose into canonical rates. Avoid changing a historical numeric DTO property to a union in place.
2. **Offer a versioned opt-in shape.** Proposed `pricing_offers` contains seller-scoped offers and discriminated components. Specify required fields and enums for each state, prohibit amounts on unpriced branches, and publish examples for a free allowance, unknown operation, scoped hosted price and additive edit. Advertise the exact version via the endpoint or media type. Keep absence on legacy records distinguishable from an offer with verified unpriced terms.
3. **Project into legacy fields only when lossless.** For new typed data use a first-party rate in a uniquely selected, explicitly documented legacy-compatible context. If currency, token direction, market or material terms would be lost, use the verified BE-02 unavailable representation. Never select the lowest offer as a default. If a legacy record conflicts with a typed offer, retain its audit trail and require a decision before enabling projection.
4. **Update derived surfaces together.** List filters, sorts, search DTOs, compare summaries, stats, catalog health and validation must use the same state and comparison rules. Define verified coverage per scoped operation and report known, free, not publicly priced, inapplicable and unknown separately. Keep the meaning of existing coverage counters documented. Version any incompatible aggregation change. Suppress unsafe comparisons in nullable legacy summaries and document the correction. OpenAPI is generated via utoipa (`server/src/routes/mod.rs:41,112,131`): check the served document, serializers and actual GET responses for required/nullable/omitted agreement.
5. **Stage rollout by verified lineage.** Pin the BE-02 head, reconcile #49, then have one owner update schema, validator, DTOs, OpenAPI and consumers. No migration is required just to read an old record. Future price additions should be small source-backed changes reviewed independently.

### Existing proposals

Backend [PR #49](https://github.com/wihlarkop/almanac/pull/49) was open at `0b2abb487cfe9bef024f3329fe9d2328688dcb22` during review. Its diff adds optional free-text `unpriced_reason`, a warning for active/deprecated records that lack both pricing and a reason, `models_by_confidence`, explanatory reasons on 25 YAMLs, and changes to `Cargo.lock`. These reasons explain a gap, but a string such as “open weights” cannot establish the price of a hosted route. Preserve those annotations as legacy display text pending scoped evidence. Keep the model-wide confidence histogram separate from per-offer evidence confidence.

The corresponding [web PR #34](https://github.com/wihlarkop/almanac-web/pull/34) was open at `d38c72080b0cf54d94480ce69d0d1e3cb63abc3e`. Its stats renderer accesses `models_by_confidence['official']` and `models_without_pricing`. Ensure both are available with their documented meanings before enabling that consumer. Reconcile the shared backend schema, model, statistics and validator changes against the eventual BE-02 head. Keep the audited mainline lockfile as the base. The typed contract does not require taking #49's lockfile or modifying either existing PR.

### Acceptance cases for implementation

| Input or scenario | Required observation |
| --- | --- |
| Existing catalog containing arbitrary historical zeroes, missing prices, or reasons from #49 | Loads without mass rewriting. Historical zero alone yields no verified-free verdict. |
| Explicitly free input tier plus billable output or operation fee | Input displays its documented zero and restrictions. The service remains partially billable. |
| Contact-sales offer, documented inapplicable meter, or 403 during evidence collection | Three distinguishable outcomes: unpublished, inapplicable, or unknown/previously verified with a fetch failure. No new verification date from a failed fetch. |
| Separate creator and hosting tariffs for one model | Both are attributed to their sellers under one canonical creator. Selecting a route cannot overwrite another route. |
| Input/output rates per million in different currencies or tiers | No global cheapest result. Matching queries can compare like units with complete qualifiers. |
| Edit references plus output megapixels, or video duration with a minimum | A total needs all components and requested dimensions. Incomplete conditions leave the total unavailable. |
| One decision request with multiple questions and no price evidence | Billing unit and price remain unknown. No token/request/decision rate is inferred. |
| Schema, validator, serializer, served OpenAPI and list/search/compare/stats | Fixtures for all states round-trip without assigning defaults. Unknown and unsupported terms stay distinct. Legacy clients can consume the documented v1 shape. |
