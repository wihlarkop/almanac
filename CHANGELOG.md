# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [Unreleased]

### Added

**Server**
- `GET /metrics` Prometheus endpoint (optional `--features metrics` cargo feature): `http_requests_total`, `http_request_duration_seconds`, `catalog_models_total`, `catalog_providers_total`, `catalog_aliases_total`. Path labels use matched route templates to avoid cardinality explosion.
- Deployment-time catalog scoping via env vars or `CATALOG_SCOPE_FILE`, with include/exclude rules for providers and models.

**Validator**
- Catalog summary report printed after every validation run: provider/model/alias totals, status breakdown, per-provider counts, and coverage stats (pricing, context window, staleness).
- `--check-urls` flag for HTTP HEAD reachability checks on all source URLs.

**Schema — new output modalities**
- `embedding` — vector embedding output (embedding models)
- `audio` — audio output (TTS, speech synthesis)
- `video` — video output (video generation models)
- `music` — music output (distinct from TTS audio; full song generation with melody, harmony, vocals)
- `3d` — 3D mesh output (`.glb`/`.obj`/`.fbx` files from 3D generation models)

**Schema — new endpoint families**
- `embeddings` — embedding generation
- `image_generation` — image synthesis
- `speech` — text-to-speech
- `transcription` — speech-to-text
- `video_generation` — video synthesis
- `realtime` — bidirectional audio/video streaming (WebSocket-based)
- `ocr` — document and image OCR
- `music_generation` — AI music and song generation
- `mesh_generation` — 3D mesh and model generation
- `reranking` — query-document relevance reranking

**Schema — new pricing fields**
- `per_image` — price per generated image
- `per_second` — price per second of audio/video output
- `per_minute` — price per minute of audio input (STT models)
- `per_million_chars` — price per 1M characters (character-priced TTS)
- `per_page` — price per page processed (OCR/document models)

**Catalog — new providers (51 added since 0.1.0, total 61)**

*LLM / Chat:*
- **Groq** — llama-3.3-70b-versatile, llama-3.1-8b-instant, llama-3.1-70b-versatile, llama-3.2-11b/90b-vision-preview, gemma2-9b-it, mixtral-8x7b-32768, llama-guard-3-8b
- **Amazon** — Nova Micro, Nova Lite, Nova Pro, Nova Premier; Titan Text Express/Lite/Premier (deprecated)
- **Microsoft** — Phi-4, Phi-4 Mini, Phi-4 Multimodal (audio added to inputs), Phi-3.5 MoE, Phi-3.5 Mini, Phi-3.5 Vision
- **Cerebras** — llama3.1-8b, llama3.3-70b
- **Baidu** — ERNIE 4.5 21B/300B, ERNIE X1, ERNIE X1 Turbo
- **Tencent** — Hunyuan A13B Instruct, Hunyuan Turbo, Hunyuan Pro, Hunyuan Lite
- **ByteDance** — Doubao 1.5 Pro 32K/256K, Doubao 1.5 Thinking Pro, Seed 2.0 Pro
- **Reka** — Reka Core, Reka Flash, Reka Edge
- **AI21 Labs** — Jamba 1.5 Large, Jamba 1.5 Mini
- **NVIDIA** — Nemotron-4-340B, Llama 3.3 Nemotron Super 49B, Llama 3.3 Nemotron Nano 8B
- **Upstage** — Solar Pro, Solar Mini
- **Inflection AI** — Inflection 3 Pi
- **Xiaomi MiMo** — MiMo V2 Pro, MiMo V2 Flash
- **MiniMax** — MiniMax M2.7, M2.5, M2.1, Text-01; Hailuo Video 01
- **Moonshot AI** — Kimi K2.5, Kimi K2.6, Moonshot V1 8K/32K/128K
- **Z.AI (Zhipu)** — GLM-4.5, GLM-4.5 Flash, GLM-4.7, GLM-4.7 Flash, GLM-5.1
- **Inception AI** — Mercury, Mercury 2, Mercury Coder (diffusion LLMs)
- **Writer** — Palmyra X5 (1M context)
- **01.AI (Yi)** — Yi Lightning, Yi Large
- **SambaNova** — Llama 3.3 70B, Llama 3.1 405B/8B
- **Naver** — HyperCLOVA X HCX-005, HCX-DASH-001 (Korean-optimized)
- **IBM** — Granite 3.3 8B/2B Instruct (Apache 2.0, 128K context, reasoning)
- **Snowflake** — Arctic Instruct (480B MoE, Apache 2.0, free via Cortex)
- **StepFun** — Step-2 (1T MoE, 256K context)

*Embedding / Reranking:*
- **Voyage AI** — voyage-4, voyage-4-lite, voyage-4-large, voyage-4-nano, voyage-code-3, voyage-finance-2, voyage-law-2, voyage-multimodal-3; rerank-2, rerank-lite-2
- **Jina AI** — jina-embeddings-v3/v4, jina-clip-v2, jina-colbert-v2; jina-reranker-v2-base-en, jina-reranker-v2-multilingual
- **Cohere** — rerank-english-v3.0 (deprecated), rerank-multilingual-v3.0 (deprecated), rerank-v3.5

*Image Generation:*
- **Stability AI** — Stable Image Ultra, Stable Image Core, Stable Diffusion 3.5 Large, SDXL
- **Black Forest Labs** — FLUX 2 Max, FLUX 2 Klein, FLUX 1.1 Pro, FLUX Kontext
- **Ideogram** — Ideogram 3.0, Ideogram 2.0
- **Adobe Firefly** — Firefly Image 5 (commercially safe)
- **Leonardo AI** — Phoenix 2

*Audio — TTS:*
- **ElevenLabs** — Eleven v3, Eleven Flash v2.5, Eleven Multilingual v2/v1; Scribe V2, Scribe V2 Realtime (STT)
- **Cartesia** — Sonic 3
- **Deepgram** — Nova 3 (STT); Aura 2 (TTS)
- **AssemblyAI** — Universal 3 Pro, Universal 2 (STT)
- **PlayHT** — Play 3.0 Ultra (voice cloning TTS)
- **LMNT** — Aurora (ultra-low-latency TTS, <100ms)

*Audio — TTS/STT additions to existing providers:*
- **Google** — Gemini 2.5 Flash TTS, Gemini 2.5 Pro TTS, Gemini 3.1 Flash TTS, Gemini 3.1 Flash Live (realtime)
- **OpenAI** — GPT-4o Realtime, GPT-4o Mini Realtime, GPT-4o Audio, GPT-4o Mini Audio, GPT-4o Transcribe, GPT-4o Mini Transcribe
- **Mistral** — Voxtral TTS, Voxtral Mini 2507 (STT)
- **xAI** — Grok STT, Grok TTS
- **Alibaba** — Qwen Audio (STT), Qwen3 TTS
- **Cohere** — Cohere Transcribe 03-2026

*Video Generation:*
- **Runway** — Gen 4.5, Gen 4.5 Turbo
- **Kling** — Kling v2.0, Kling v3.0
- **Luma AI** — Ray 3.14
- **Pika** — Pika v2.5
- **Haiper** — Haiper Video 2
- **Lightricks** — LTX Video (fast open-weight)
- **Vidu** — Vidu 2 (Shengshu Technology)
- **HeyGen** — Avatar 4 (AI talking-head avatar video)
- **Synthesia** — Synthesia 1 (enterprise avatar video)
- **PixVerse** — PixVerse V4

*Video additions to existing providers:*
- **OpenAI** — Sora
- **Google** — Veo 2.0 (deprecating), Veo 3.1 generate/fast/lite
- **Alibaba** — Wan 2.1
- **xAI** — Grok Imagine (image), Grok Imagine Video
- **MiniMax** — Hailuo Video 01

*Music Generation (new modality):*
- **Suno** — Suno V4 (full song generation with vocals)
- **Udio** — Udio V2 (music generation with audio conditioning)

*3D Generation (new modality):*
- **Meshy** — Meshy 4 (text/image → `.glb`/`.fbx` with PBR textures)
- **Tripo AI** — Tripo 3D (3D generation with rigging support)

*Embedding additions:*
- **Google** — Gemini Embedding 001, Gemini Embedding 2 (multimodal), Text Embedding 005
- **Amazon** — Titan Embed Text v1 (deprecated), Titan Embed Text v2
- **OpenAI** — text-embedding-3-small, text-embedding-3-large, text-embedding-ada-002

*OCR:*
- **Mistral** — Mistral OCR 3 ($0.001/page)

*Image generation additions:*
- **Google** — Imagen 4.0, Imagen 4.0 Fast
- **Amazon** — Titan Image Generator v2
- **OpenAI** — GPT Image 1, GPT Image 1 Mini
- **xAI** — Grok Imagine (image quality tier)

*OpenAI — o-series completions:*
- o1-pro ($150/$600 per 1M, Responses API, ChatGPT Pro tier)

**Aliases**
- 228 total aliases (up from ~80 at 0.1.0)

### Changed
- `ureq` upgraded from 2.x to 3.3.0; updated `AgentBuilder` → `Agent::config_builder`, `Error::Status` → `Error::StatusCode`
- Workspace `Cargo.toml` dependencies reorganized into named groups (error handling, async runtime, web framework, serialization, etc.)
- `phi-4-multimodal`: `input` expanded from `[text, image]` to `[text, image, audio]`
- Removed redundant `tower` entry from server `[dev-dependencies]`

---

## [0.1.0] - 2026-05-09

### Added

**API server**
- `GET /api/v1/health` — server health and version
- `GET /api/v1/providers` — list all providers with ETag caching
- `GET /api/v1/providers/{id}` — single provider detail
- `GET /api/v1/models` — paginated model list with filtering (`provider`, `status`, `capability`, `modality_input`, `modality_output`, `min_context`, `max_input_price`), sorting (`provider`, `id`, `status`, `context_window`, `max_output_tokens`), and offset pagination
- `GET /api/v1/models/{provider}/{id}` — single model detail
- `GET /api/v1/aliases` — list all shorthand aliases
- `GET /api/v1/aliases/{alias}` — resolve a single alias to its canonical model id
- `POST /api/v1/validate` — validate a model string; returns canonical id, errors, and fuzzy suggestions
- `GET /api/v1/search` — full-text search across model ids, display names, and provider names
- `GET /api/v1/suggest` — ranked fuzzy suggestions for a partial model string
- `GET /api/v1/compare` — side-by-side comparison of two models
- `GET /api/v1/catalog/health` — catalog freshness and coverage stats (total models, missing pricing, stale records)
- `GET /api/v1/catalog/issues` — detailed catalog data quality issues

**Response contract**
- Uniform `ApiResponse<T>` envelope on every endpoint: `success`, `message`, `data`, `meta`, `error`
- `meta` includes `request_id`, `execution_time_seconds`, `timestamp`, and pagination fields
- Standard error codes: `MODEL_NOT_FOUND`, `PROVIDER_NOT_FOUND`, `ALIAS_NOT_FOUND`, `BAD_REQUEST`, `RATE_LIMIT_EXCEEDED`, `REQUEST_TIMEOUT`, `PAYLOAD_TOO_LARGE`, `INTERNAL_SERVER_ERROR`
- ETag + `Cache-Control: public, max-age=300` on catalog endpoints
- `X-Request-Id` propagation
- Security headers on all responses (`X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`)

**Infrastructure**
- `LOG_FORMAT=json` env var for structured JSON logging in production
- `RATE_LIMIT_RPS` env var for per-IP rate limiting (disabled by default)
- `PORT` and `DATA_DIR` env vars for server configuration
- Graceful shutdown on `SIGTERM` and `Ctrl+C`
- SIGHUP catalog hot-reload (Unix only)
- Docker image with non-root user and embedded catalog
- OpenAPI spec served at `/openapi.json`; interactive docs at `/swagger-ui` and `/scalar`

**Catalog**
- 201 model entries across 10 providers: Anthropic, OpenAI, Google, Meta, Mistral, Cohere, DeepSeek, xAI, Alibaba (Qwen), Perplexity
- Provider metadata in `providers/`
- Alias resolution in `aliases.yaml`
- Per-model fields: id, provider, display_name, status, lifecycle dates, context_window, max_output_tokens, modalities, capabilities, parameters, pricing, confidence, endpoint_family, sources, last_verified

**Validator** (`cargo run -p almanac-validator`)
- Schema validation for all provider and model YAML files
- Filename/id consistency checks
- Provider reference checks
- Alias chain and shadow detection
- Lifecycle date ordering and replacement checks
- Parameter overlap checks
- Freshness and pricing coverage report
- `--check-urls` flag for optional source URL reachability checks

---

## Versioning policy

This project uses [Semantic Versioning](https://semver.org/).

- **Patch** — bug fixes, data corrections, new model entries, dependency updates
- **Minor** — new endpoints, new response fields (additive), new query parameters
- **Major** — breaking changes to existing response shapes, removed endpoints, renamed fields
