# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.1.0] - 2026-05-12

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
- `GET /metrics` Prometheus endpoint (optional `--features metrics` cargo feature): `http_requests_total`, `http_request_duration_seconds`, `catalog_models_total`, `catalog_providers_total`, `catalog_aliases_total`. Path labels use matched route templates to avoid cardinality explosion.

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
- Deployment-time catalog scoping via env vars or `CATALOG_SCOPE_FILE`, with include/exclude rules for providers and models.

**Validator** (`cargo run -p almanac-validator`)
- Schema validation for all provider and model YAML files
- Filename/id consistency checks
- Provider reference checks
- Alias chain and shadow detection
- Lifecycle date ordering and replacement checks
- Parameter overlap checks
- Freshness and pricing coverage report
- `--check-urls` flag for optional source URL reachability checks
- Catalog summary report printed after every validation run: provider/model/alias totals, status breakdown, per-provider counts, and coverage stats (pricing, context window, staleness).

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

**Catalog — 59 providers, 400+ model entries**

*Core providers (initial):*
- **Anthropic** — Claude 1–3.7, Claude 3.5 Haiku/Sonnet, Claude Opus/Sonnet/Haiku 4-series
- **OpenAI** — GPT-3.5/4/4o/4.1/4.5 families; o1/o3/o4 reasoning series; GPT-5 family; DALL-E 2/3; Sora; Whisper; TTS-1; Realtime/Audio/Transcribe variants; GPT Image 1
- **Google** — Gemini 1.0–3.1 (Flash/Pro/Flash-Lite/TTS/Live); Gemma 3; Imagen 4; Veo 2–3.1; Gemini Embedding; LearnLM
- **Meta** — Llama 2/3/3.1/3.2/3.3/4 (Scout, Maverick); Llama Guard; Code Llama
- **Mistral** — Mistral 7B/Large/Medium/Small/Saba; Mixtral 8x7B/8x22B; Codestral; Pixtral; Ministral; Magistral; Devstral; Voxtral; Mistral OCR 3
- **Cohere** — Command R/R+/R7B/A series; Aya Expanse/Vision; Embed v3/v4; Rerank v3/v3.5; Cohere Transcribe; Tiny Aya variants
- **DeepSeek** — DeepSeek V3/V4/R1 families; DeepSeek Reasoner
- **xAI** — Grok 2/3/4 families; Grok Vision/Audio/TTS/STT/Imagine
- **Alibaba** — Qwen2.5/Qwen3 (7B–235B); Qwen3.5/3.6 Omni; Qwen Audio/TTS; Wan 2.1 (video)
- **Perplexity** — Sonar, Sonar Pro, Sonar Reasoning Pro, Sonar Deep Research

*LLM / Chat (new providers):*
- **Amazon** — Nova Micro/Lite/Pro/Premier/2-Lite; Nova Sonic/Nova 2 Sonic; Titan Text/Image/Embed (deprecated variants)
- **Microsoft** — Phi-4, Phi-4 Mini, Phi-4 Multimodal, Phi-3.5 MoE/Mini/Vision; MAI Transcribe/Voice/Image
- **Baidu** — ERNIE 4.5 21B/300B, ERNIE X1/X1 Turbo, ERNIE 5.0, ERNIE 5.1
- **Tencent** — Hunyuan A13B Instruct, Hunyuan Turbo/Pro/Lite, Hy3 Preview
- **ByteDance** — Doubao 1.5 Pro 32K/256K, Doubao 1.5 Thinking Pro, Seed 2.0 Pro/Mini/Lite
- **Reka** — Reka Core, Reka Flash, Reka Edge, Reka Flash 3, Reka Edge 2603
- **AI21 Labs** — Jamba 1.5 Large (deprecated), Jamba 1.5 Mini (deprecated), Jamba Large 1.7, Jamba Mini 1.7
- **NVIDIA** — Nemotron-4-340B, Llama 3.3 Nemotron Super 49B/Nano 8B, Nemotron-3 Super 120B, Nemotron-3 Nano Omni 30B
- **Upstage** — Solar Pro, Solar Mini
- **Inflection AI** — Inflection 3 Pi
- **Xiaomi MiMo** — MiMo V2 Pro, MiMo V2 Flash
- **MiniMax** — MiniMax M2.1/M2.5/M2.7, Text-01
- **Moonshot AI** — Kimi K2/K2.5/K2.6, Moonshot V1 8K/32K/128K
- **Z.AI (Zhipu)** — GLM-4.5, GLM-4.5 Flash, GLM-4.7, GLM-4.7 Flash, GLM-5, GLM-5.1
- **Inception AI** — Mercury, Mercury 2, Mercury Coder (diffusion LLMs)
- **Writer** — Palmyra X5 (1M context)
- **01.AI (Yi)** — Yi Lightning, Yi Large
- **Naver** — HyperCLOVA X HCX-005, HCX-DASH-001 (Korean-optimized)
- **IBM** — Granite 3.3 8B/2B Instruct (Apache 2.0, 128K context, reasoning)
- **Snowflake** — Arctic Instruct (480B MoE, Apache 2.0)
- **StepFun** — Step-2 (1T MoE, 256K context)
- **Fireworks AI** — FireFunction V2 (fine-tuned function-calling model)

*Embedding / Reranking:*
- **Voyage AI** — voyage-3/4/4-lite/4-large/4-nano/code-3/finance-2/law-2/multimodal-3; rerank-2, rerank-lite-2
- **Jina AI** — jina-embeddings-v3/v4, jina-clip-v2, jina-colbert-v2; jina-reranker-v2-base-en/multilingual
- **Cohere** — rerank-english-v3.0 (deprecated), rerank-multilingual-v3.0 (deprecated), rerank-v3.5

*Image Generation:*
- **Stability AI** — Stable Image Ultra, Stable Image Core, Stable Diffusion 3.5 Large, SDXL
- **Black Forest Labs** — FLUX 2 Max, FLUX 2 Klein, FLUX 1.1 Pro, FLUX Kontext
- **Ideogram** — Ideogram 2.0, Ideogram 3.0
- **Adobe Firefly** — Firefly Image 5 (commercially safe)
- **Leonardo AI** — Phoenix 2

*Audio — TTS / STT:*
- **ElevenLabs** — Eleven v3, Eleven Flash v2.5, Eleven Multilingual v2/v1; Scribe V2, Scribe V2 Realtime (STT)
- **Cartesia** — Sonic 3
- **Deepgram** — Nova 2/Nova 3 (STT); Aura 2 (TTS)
- **AssemblyAI** — Universal 2, Universal 3 Pro (STT)
- **PlayHT** — Play 3.0 Ultra (voice cloning TTS)
- **LMNT** — Aurora (ultra-low-latency TTS, <100ms)

*Video Generation:*
- **Runway** — Gen 4, Gen 4.5
- **Kling** — Kling v2.0, Kling v3.0
- **Luma AI** — Ray 3.14
- **Pika** — Pika v2.5
- **Haiper** — Haiper Video 2
- **Lightricks** — LTX Video (fast open-weight)
- **Vidu** — Vidu 2 (Shengshu Technology)
- **HeyGen** — Avatar 4 (AI talking-head avatar video)
- **Synthesia** — Synthesia 1 (enterprise avatar video)
- **PixVerse** — PixVerse V4

*Music Generation:*
- **Suno** — Suno V4 (full song generation with vocals)
- **Udio** — Udio V2 (music generation with audio conditioning)

*3D Generation:*
- **Meshy** — Meshy 4 (text/image → `.glb`/`.fbx` with PBR textures)
- **Tripo AI** — Tripo 3D (3D generation with rigging support)

**Aliases**
- 228 shorthand aliases resolving to canonical provider/model pairs

### Changed
- `ureq` upgraded from 2.x to 3.3.0; updated `AgentBuilder` → `Agent::config_builder`, `Error::Status` → `Error::StatusCode`
- Workspace `Cargo.toml` dependencies reorganized into named groups (error handling, async runtime, web framework, serialization, etc.)
- `phi-4-multimodal`: `input` expanded from `[text, image]` to `[text, image, audio]`
- `grok-4-1-fast`: status `active` → `deprecated`; deprecation_date 2026-05-06, sunset_date 2026-05-15, replacement `grok-4.3`
- `jamba-1.5-large`: status `active` → `deprecated`; deprecation_date 2025-08-27, sunset_date 2026-02-27, replacement `jamba-large-1.7`
- `jamba-1.5-mini`: status `active` → `deprecated`; deprecation_date 2025-08-27, sunset_date 2026-02-27, replacement `jamba-mini-1.7`
- Removed redundant `tower` entry from server `[dev-dependencies]`

---

## Versioning policy

This project uses [Semantic Versioning](https://semver.org/).

- **Patch** — bug fixes, data corrections, new model entries, dependency updates
- **Minor** — new endpoints, new response fields (additive), new query parameters
- **Major** — breaking changes to existing response shapes, removed endpoints, renamed fields
