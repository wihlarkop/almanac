# API performance baseline

Measured on 2026-11-18 from [`2d4278721b8899a26971a8db3a78cb37e972fb01`](https://github.com/wihlarkop/almanac/commit/2d4278721b8899a26971a8db3a78cb37e972fb01) for [BE-14](https://github.com/wihlarkop/almanac/issues/68). The release binary was built with the pinned Rust 1.96.0 toolchain (`x86_64-pc-windows-msvc`) from the existing lockfile. `/api/v1/health` reported version `0.27.0`, 817 models, 62 providers, and 292 aliases in all five startup samples.

## Environment and workloads

- Windows build 26200 x64, Intel Core i7-9750H (12 logical processors), 15.84 GiB host RAM, Windows PowerShell 5.1, Node.js v24.20.0. Release profile uses `opt-level = "z"`, LTO, and one codegen unit. Metrics feature off.
- HTTP/1.1 over `127.0.0.1`, one kept alive socket and at most one in-flight request. No catalog scoping or Redis for the main workload. Rate limiting disabled for timings and enabled separately for the fallback check. Request completion logs suppressed below warn. The server binds all interfaces in this revision, so run on an isolated development host.
- Each route gets 20 warmups then 200 timed requests per cache configuration. Client latency starts before request construction and ends after reading the response body. Percentiles use nearest rank for each set of 200. Body bytes are the last sample, before JSON parsing and excluding HTTP headers. Response metadata can vary byte counts slightly.
- Start each run from a new server process. Startup duration is from before child spawn through the first successful health response after the listening log. Catalog duration is the difference between the server's `starting almanac server` and `catalog loaded` log timestamps, including scope application, index creation, hashing and stats. Memory is a point-in-time `Get-Process` snapshot after readiness and after the timings. Operating system file caches and host load were uncontrolled. An initial pilot hit a 25-second startup deadline before the catalog-loaded log. The recorded run used a 120-second deadline.

## Startup and memory

| Process | Ready (ms) | Catalog (ms) | Working set (MiB) | Private bytes (MiB) |
| --- | ---: | ---: | ---: | ---: |
| 1 | 6,297.3 | 6,184.4 | 13.01 | 7.55 |
| 2 | 422.3 | 349.4 | 12.96 | 7.41 |
| 3 | 427.1 | 364.4 | 12.91 | 7.39 |
| 4 | 426.5 | 367.2 | 12.90 | 7.31 |
| 5 | 380.8 | 318.5 | 12.96 | 7.46 |

The first captured process accounts for most of the spread. It was not a controlled cold boot. The five-sample median was 426.5 ms to readiness and 364.4 ms for catalog setup. Working set and private bytes at the end of the endpoint runs were 14.54 and 8.68 MiB with the cache disabled, and 13.82 and 8.21 MiB with the memory cache. These snapshots do not measure peak memory or the cost of many distinct cache keys.

## Endpoint latency and payloads

Paths below are relative to `/api/v1`. All responses use `Accept-Encoding: identity` here.

| Request | Status | Entries | `none` p50 / p95 (ms) | `none` bytes | Warm `memory` p50 / p95 (ms) | `memory` bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `GET /models` | 200 | 20 | 7.305 / 10.151 | 16,753 | 0.708 / 1.531 | 16,741 |
| `GET /models?provider=openai&status=active&limit=20` | 200 | 20 | 1.844 / 2.628 | 16,686 | 0.704 / 1.415 | 16,684 |
| `GET /models?limit=20&offset=400` | 200 | 20 | 4.952 / 7.152 | 15,406 | 0.698 / 1.477 | 15,404 |
| `GET /models?limit=1000` | 200 | 100 | 5.965 / 8.810 | 83,115 | 0.908 / 1.390 | 83,112 |
| `POST /validate` known `gpt-4o` | 200 | — | 0.518 / 0.758 | 242 | 0.711 / 2.260 | 243 |
| `POST /validate` unknown `gpt4o-mini` | 422 | — | 4.051 / 5.239 | 415 | 5.019 / 6.195 | 415 |
| `GET /suggest?q=gpt4o-mini&limit=5` | 200 | 5 | 4.505 / 5.770 | 762 | 0.437 / 0.622 | 761 |
| `GET /compare?models=openai/gpt-4o,openai/gpt-4o-mini` | 200 | 2 | 0.473 / 0.779 | 2,639 | 0.430 / 0.718 | 2,639 |

`/models` returned `total_data=817`. The filtered request returned 68 matches before pagination. `limit=1000` was clamped to 100 and `offset=99999` became 817 with an empty page. `/suggest` reported 21 matches before truncating to five. With caching disabled, requesting gzip returned 2,350 bytes for the default page and 8,016 for the 100-entry page. The two validation requests bypass caching in both runs. The GET hit timings are an observation on a small fixed set of warm keys and do not justify enabling memory caching with the correctness issues below.

## Cache and conditional requests

| Probe | `CACHE_BACKEND=none` | `CACHE_BACKEND=memory` |
| --- | --- | --- |
| First `GET /models` | 200 `MISS` | 200 `MISS` |
| Same URI and matching `If-None-Match` | 304, empty body | 200 `HIT` with cached body |
| Matching token on an uncached page key | 304, empty body | 304, empty body |
| Reordered distinct parameters on a warmed key | 200 `MISS` | 200 `HIT` |
| Different limit on the same filtered list | 200 `MISS` | 200 `MISS` |

- Successful list responses carried `ETag`, `Cache-Control: public, max-age=300, stale-while-revalidate=60`, and `Vary` including `Accept-Encoding`. The same middleware sets `Last-Modified` from the catalog load time. Observed 304s omitted `ETag` and `Cache-Control`. Health is outside the application body cache but the general response-header middleware still applies to successful GETs. [Header middleware](../../server/src/request.rs#L390), [cache middleware](../../server/src/cache.rs#L171).
- The body cache looks up an entry using the catalog ETag, path, and hash of sorted raw query pairs before calling the route. A hit bypasses the route's `If-None-Match` check. Unencoded JSON is stored and gzip is applied outside the body cache. The no-op backend still hashes, collects, and copies eligible 200 bodies before discarding the cache entry. [Router layers](../../server/src/routes/mod.rs#L74), [cache implementation](../../server/src/cache.rs#L154), [conditional list response](../../server/src/routes/models.rs#L220).
- The ETag was shared by different list queries in each process and changed across unchanged catalog launches (`dec184da…`, `e4913e6c…`, `9e3cea4d…`, `8c544b7d…`). The hash serializes scoped models and providers directly. Model capabilities are a `HashMap`, so iteration order is not canonical across loads. Aliases are excluded from the hash input even though alias, suggestion and related endpoints depend on them. The measurements establish unstable tags on this baseline. The source shows aliases do not influence the hash directly. [State construction](../../server/src/state.rs#L44), [model fields](../../server/src/catalog.rs#L14).
- Cached JSON includes request-specific envelope metadata from the miss. The outer request context still stamps a fresh response header. The in-memory map has no entry or size cap and clears expired entries only when those keys are looked up. These are source findings. [Cache entry storage](../../server/src/cache.rs#L30), [request context](../../server/src/request.rs#L36).

## Rate limiting, Redis, and operations

- With `RATE_LIMIT_RPS=2` and the default memory backend, three requests after a 1.1-second pause returned `200, 200, 429` with remaining counts `1, 0` on the first two. The 429 carried `RATE_LIMIT_EXCEEDED` and `Retry-After: 1`. After another 1.1 seconds the next request returned 200 with one remaining. That process reached readiness in 1.28 seconds.
- With both backends set to `redis` and `REDIS_URL` on an unused loopback port, readiness took 34.94 seconds from spawn. Logs reported `redis cache connection failed; caching disabled` and `redis rate limiter failed; falling back to memory`. The request burst and recovery matched the memory baseline. Successful catalog reads showed `X-Cache: MISS`. Startup initializes cache and rate-limit Redis connections serially before listening. The active Redis limiter's command-error path allows requests, and cache read errors are treated as misses. These two runtime error paths were inspected in source. A connected Redis instance and a connection loss after startup were not exercised. [Startup backend selection](../../server/src/main.rs#L148), [Redis limiter](../../server/src/request.rs#L218), [Redis cache](../../server/src/cache.rs#L57).
- `TRUST_PROXY_HEADERS` defaults off, so this direct loopback workload used socket peer IP. Opting in reads `X-Forwarded-For` or `X-Real-IP` without checking the peer against a configured trusted-proxy list. Only enable it where ingress is restricted to a trusted proxy that replaces those headers. Deployment proxy behavior was reviewed in source. [IP selection](../../server/src/request.rs#L344).
- The startup probe used the built-in health endpoint. The optional `/metrics` route and catalog gauges were inspected in source. Unix SIGHUP reload, SIGTERM shutdown, Redis operation under load, Docker and hosted platform performance were not measured by this Windows process harness. [Metrics registration](../../server/src/metrics.rs#L27), [Unix reload and shutdown](../../server/src/main.rs#L72).

## Follow-up work

Preserve the existing catalog API while evaluating sorting model references before cloning only the requested page. The current list handler clones all matching records before sorting and slicing. Check ordering, ties and pagination against the same workload. Coordinate this change with the model route owner after the metadata contract lands. For response caching, give conditional requests a consistent path on a cache hit, separate request metadata from cached content, and canonicalize fingerprints with alias coverage before evaluating shared Redis cache effectiveness. Set explicit Redis initial connection and request budgets when the availability policy is decided. [Model list handler](../../server/src/routes/models.rs#L203).

## Reproduce on Windows

Use a clean worktree at the pinned source commit. Place the script below in a scratch `api-benchmark.cjs` file outside the worktree, then pass the worktree as its argument. It isolates the child process environment and reads the tracked catalog directly from that worktree.

```powershell
git rev-parse HEAD
rustc -Vv
node --version
cargo build --locked --offline --release -p almanac-server
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$benchmark = 'C:\path\to\api-benchmark.cjs'
node --check $benchmark
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
node $benchmark (Get-Location).Path
```

Use the same pinned lockfile when populating the Cargo cache for a first build. The offline build and benchmark above completed successfully. The benchmark emits JSON lines with startup timings, latency percentiles, response sizes, cache headers, process memory, and fallback receipts. It does not time the build itself. Build and loopback results describe this Windows host. Rust unit tests, Clippy, formatter, validator, security audit, Docker, and hosted checks are separate gates.

<details>
<summary>Benchmark harness (Node.js)</summary>

```javascript
const assert = require('node:assert/strict');
const { spawn, execFileSync } = require('node:child_process');
const fs = require('node:fs');
const http = require('node:http');
const net = require('node:net');
const os = require('node:os');
const path = require('node:path');
const { once } = require('node:events');
const { setTimeout: sleep } = require('node:timers/promises');

const root = path.resolve(process.argv[2] || '.');
const exe = path.join(root, 'target', 'release', 'server.exe');
const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'almanac-api-bench-'));
fs.writeFileSync(path.join(scratch, '.env'), '# isolated benchmark\n');

async function freePort() {
  const socket = net.createServer();
  socket.listen(0, '127.0.0.1');
  await once(socket, 'listening');
  const { port } = socket.address();
  socket.close();
  await once(socket, 'close');
  return port;
}

async function withServer(overrides, use) {
  const port = await freePort();
  const env = {};
  for (const key of ['SystemRoot', 'PATH', 'TEMP', 'TMP']) {
    if (process.env[key]) env[key] = process.env[key];
  }
  Object.assign(env, {
    PORT: String(port), DATA_DIR: root, LOG_FORMAT: 'json',
    RUST_LOG: 'info,almanac_server::request=warn',
    CACHE_BACKEND: 'none', CACHE_TTL_SECS: '300',
    RATE_LIMIT_BACKEND: 'memory', TRUST_PROXY_HEADERS: 'false',
  }, overrides);
  const start = performance.now();
  const child = spawn(exe, [], { cwd: scratch, env, windowsHide: true });
  const agent = new http.Agent({ keepAlive: true, maxSockets: 1 });
  let stdout = '';
  let stderr = '';
  let error;
  child.on('error', (e) => { error = e; });
  child.stderr.on('data', (bytes) => { stderr += bytes; });
  child.stdout.on('data', (bytes) => { stdout += bytes; });
  const closed = new Promise((resolve) => child.once('close', resolve));
  const logs = () => stdout.split('\n').filter(Boolean).flatMap((line) => {
    try { return [JSON.parse(line)]; } catch { return []; }
  });
  const request = (endpoint, { body, headers = {} } = {}) => new Promise((resolve, reject) => {
    const t = performance.now();
    const payload = body === undefined ? undefined : JSON.stringify(body);
    const req = http.request({
      hostname: '127.0.0.1', port, path: '/api/v1' + endpoint, agent,
      method: payload === undefined ? 'GET' : 'POST',
      headers: {
        'accept-encoding': 'identity',
        ...(payload === undefined ? {} : { 'content-type': 'application/json' }),
        ...headers,
      },
    }, (res) => {
      const chunks = [];
      res.on('data', (chunk) => chunks.push(chunk));
      res.on('error', reject);
      res.on('end', () => resolve({
        ms: performance.now() - t, code: res.statusCode,
        headers: res.headers, body: Buffer.concat(chunks),
      }));
    });
    req.setTimeout(10000, () => req.destroy(new Error('request timed out')));
    req.on('error', reject);
    req.end(payload);
  });
  try {
    while (!logs().some(({fields}) => fields.message === 'server listening')) {
      if (error || child.exitCode !== null || performance.now() - start > 120000) {
        throw Error('server startup: ' + (error || child.exitCode) + '\n' + stdout + '\n' + stderr);
      }
      await sleep(5);
    }
    const health = await request('/health');
    const ready = performance.now() - start;
    assert.equal(health.code, 200);
    const readTime = (message) => {
      const iso = logs().find(({fields}) => fields.message === message).timestamp;
      const [whole, fractional] = iso.split('.');
      return Date.parse(whole + 'Z') + Number('0.' + fractional.replace('Z', '')) * 1000;
    };
    const load = readTime('catalog loaded') - readTime('starting almanac server');
    const memory = () => JSON.parse(execFileSync('powershell.exe', [
      '-NoProfile', '-Command',
      '$p=Get-Process -Id ' + child.pid + '; @{ws=$p.WorkingSet64;private=$p.PrivateMemorySize64}|ConvertTo-Json -Compress',
    ], { windowsHide: true }).toString());
    return await use({ request, ready, load, health, memory, logs });
  } finally {
    agent.destroy();
    child.kill();
    await closed;
  }
}

const jobs = [
  ['models default', '/models', {}, 200],
  ['models filtered', '/models?provider=openai&status=active&limit=20', {}, 200],
  ['models offset', '/models?limit=20&offset=400', {}, 200],
  ['models cap', '/models?limit=1000', {}, 200],
  ['validate known', '/validate', {body:{model:'gpt-4o',provider:'openai'}}, 200],
  ['validate missing', '/validate', {body:{model:'gpt4o-mini'}}, 422],
  ['suggest', '/suggest?q=gpt4o-mini&limit=5', {}, 200],
  ['compare', '/compare?models=openai/gpt-4o,openai/gpt-4o-mini', {}, 200],
];
const percentile = (ordered, p) => Number(ordered[Math.ceil(p * ordered.length) - 1].toFixed(3));
const brief = (r) => ({
  code: r.code, bytes: r.body.length, xCache: r.headers['x-cache'] || null,
  etag: r.headers.etag || null, cacheControl: r.headers['cache-control'] || null,
  vary: r.headers.vary || null, encoding: r.headers['content-encoding'] || 'identity',
});
const parse = (r) => JSON.parse(r.body.toString());
async function timings(request) {
  const rows = [];
  for (const [name, endpoint, opts, code] of jobs) {
    const run = async () => {
      const r = await request(endpoint, opts);
      assert.equal(r.code, code, name + ': ' + r.body.toString().slice(0, 300));
      return r;
    };
    for (let i = 0; i < 20; ++i) await run();
    const times = [];
    let last;
    for (let i = 0; i < 200; ++i) {
      last = await run();
      times.push(last.ms);
    }
    times.sort((a, b) => a - b);
    const {data, meta} = parse(last);
    rows.push({
      name, code, cache: last.headers['x-cache'] || null,
      p50ms: percentile(times, .5), p95ms: percentile(times, .95), bytes: last.body.length,
      n: Array.isArray(data) ? data.length : (Array.isArray(data.models) ? data.models.length : undefined),
      limit: meta.limit, offset: meta.offset, total: meta.total_data,
    });
  }
  return rows;
}

async function run() {
  const machine = {
    os: os.release(), arch: os.arch(), ramGiB: os.totalmem()/2**30,
    cpu: os.cpus()[0].model, threads: os.availableParallelism(), node: process.version,
  };
  console.log(JSON.stringify({machine, utc: new Date().toISOString()}));
  for (let i = 0; i < 5; ++i) {
    await withServer({}, async ({ready, load, health, memory}) => {
      console.log(JSON.stringify({startupRun: i + 1, ready, load, catalog: parse(health).data, ...memory()}));
    });
  }
  for (const backend of ['none', 'memory']) {
    await withServer({CACHE_BACKEND: backend}, async ({request, memory}) => {
      const first = await request('/models', {headers:{'x-request-id':'bench-first'}});
      const etag = first.headers.etag;
      assert.match(etag, /^"[a-f0-9]{64}"$/);
      const conditional = await request('/models', {headers:{'if-none-match':etag}});
      const uncachedConditional = await request('/models?limit=19', {headers:{'if-none-match':etag}});
      const pageA = await request('/models?provider=openai&limit=5');
      const pageReordered = await request('/models?limit=5&provider=openai');
      const pageDifferent = await request('/models?limit=6&provider=openai');
      const page400 = await request('/models?limit=20&offset=400');
      const pastEnd = await request('/models?limit=20&offset=99999');
      const gzip20 = await request('/models', {headers:{'accept-encoding':'gzip'}});
      const gzip100 = await request('/models?limit=1000', {headers:{'accept-encoding':'gzip'}});
      const samples = await timings(request);
      console.log(JSON.stringify({
        backend, first: brief(first), conditional: brief(conditional),
        uncachedConditional: brief(uncachedConditional), pageA: brief(pageA),
        pageReordered: brief(pageReordered), pageDifferent: brief(pageDifferent),
        page400: brief(page400), pastEnd: {n:parse(pastEnd).data.length,...parse(pastEnd).meta},
        gzip20: brief(gzip20), gzip100: brief(gzip100), samples, endMemory:memory(),
      }));
    });
  }
  const redisPort = await freePort();
  for (const [label, env] of [
    ['memory rate limiting', {RATE_LIMIT_RPS:'2'}],
    ['redis refused', {CACHE_BACKEND:'redis', RATE_LIMIT_BACKEND:'redis', RATE_LIMIT_RPS:'2',
      REDIS_URL:'redis://127.0.0.1:' + redisPort}],
  ]) {
    await withServer(env, async ({request, logs, ready}) => {
      const describe = r => ({
        ...brief(r), remaining: r.headers['x-ratelimit-remaining'] || null,
        retryAfter: r.headers['retry-after'] || null, errorCode: parse(r).error?.code || null,
      });
      await sleep(1100);
      const burst = [];
      for (let i = 0; i < 3; ++i) burst.push(describe(await request('/models')));
      await sleep(1100);
      const recovery = describe(await request('/models'));
      console.log(JSON.stringify({label, ready, burst, recovery,
        warnings:logs().filter(l => l.level === 'WARN').map(l => l.fields.message)}));
    });
  }
}
run().catch(error => { console.error(error); process.exitCode = 1; })
  .finally(() => { fs.unlinkSync(path.join(scratch, '.env')); fs.rmdirSync(scratch); });
```

</details>
