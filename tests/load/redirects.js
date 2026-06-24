/**
 * tests/load/redirects.js
 *
 * Load test: URL redirect hot path  (GET /{short_code} → HTTP 307)
 *
 * This test exercises the full three-tier cache stack:
 *   L1: Moka in-process cache (3s TTL, 1M entries) — sub-millisecond
 *   L2: Redis                                       — ~1ms
 *   L3: PostgreSQL                                  — ~5ms
 *
 * Strategy
 * ─────────
 * 1. setup()   — Signs up a test user via OTP (MODE=development), creates a
 *                permanent short URL, and returns its short_code. The code
 *                stays in the DB for the full test duration.
 *
 * 2. default() — Every VU hammers GET /{short_code} with redirects:0 to assert
 *                the 307 directly without following the redirect.
 *
 * 3. teardown()— Deletes the test URL from the DB to keep the DB clean.
 *
 * Why thresholds are set the way they are
 * ────────────────────────────────────────
 * This is a local Kind cluster (single node). The full request path is:
 *   k6 → host network → kind port mapping → NGINX Ingress → ClusterIP → url-service
 * Each hop adds ~1–3ms. Under 500 VUs the OS kernel's TCP/accept queue fills
 * and adds queuing latency. Real thresholds for a single-node dev cluster:
 *   p50 < 20ms   (usually 2–5ms on L1/L2 hit, but queuing adds up)
 *   p99 < 200ms  (worst-case queue drain under 500 VUs spike)
 *
 * For production EKS/GKE with multiple nodes and external LB, expect:
 *   p50 < 5ms, p99 < 15ms
 *
 * Run (local):
 *   k6 run tests/load/redirects.js
 *
 * Run with a pre-existing code:
 *   k6 run -e SHORT_CODE=abc123 tests/load/redirects.js
 *
 * Run inside kind cluster (direct to url-service, bypasses NGINX overhead):
 *   kubectl run k6 --rm -i --restart=Never --image=grafana/k6 -- run \
 *     -e BASE_URL=http://url-service.bittuly.svc.cluster.local:3002 \
 *     - < tests/load/redirects.js
 */

import http from 'k6/http';
import { check, sleep, fail } from 'k6';
import { Counter, Trend, Rate } from 'k6/metrics';

// ── Custom metrics ──────────────────────────────────────────────────────────
const redirectHits   = new Counter('redirect_hits');
const redirectMisses = new Counter('redirect_misses');
const redirectLat    = new Trend('redirect_latency_ms', true);
const errorRate      = new Rate('redirect_error_rate');
const rateLimitRate  = new Rate('rate_limited_rate');

// ── Config ──────────────────────────────────────────────────────────────────
const BASE_URL   = __ENV.BASE_URL   || 'http://localhost:8000';
const AUTH_URL   = __ENV.AUTH_URL   || BASE_URL;
const SHORT_CODE = __ENV.SHORT_CODE || '';   // If set, skip setup() auth/create

const JSON_HEADERS = { 'Content-Type': 'application/json' };

// ── Load profile ─────────────────────────────────────────────────────────────
// NOTE: These thresholds reflect the full NGINX → kind → url-service stack on
// a single local machine. They are intentionally wider than production SLOs.
//
// NGINX rate limit: 2000 req/s with burst x5 = ~10000 burst.
// At 300 VUs the system hits the rate limit. 429s from NGINX are EXPECTED and
// are NOT counted as failures — they're tracked separately in rate_limited_rate.
//
// For production benchmarks, run k6 from inside the cluster directly against
// the ClusterIP to bypass NGINX (see header comment above).
export const options = {
  stages: [
    { duration: '15s', target: 50  }, // Ramp — warms up L1/L2 cache
    { duration: '30s', target: 200 }, // Sustained load (within rate limit)
    { duration: '20s', target: 300 }, // Spike (will trigger 429s)
    { duration: '20s', target: 200 }, // Recovery
    { duration: '15s', target: 0   }, // Ramp-down
  ],
  thresholds: {
    // Local Kind single-node SLO (NGINX included)
    'http_req_duration':   ['p(99)<200', 'p(50)<20'],
    // Only count actual server errors (5xx) as failures — NOT 429s
    // k6 marks non-2xx AND non-3xx as failed, so we use a custom metric instead
    'redirect_error_rate': ['rate<0.01'],
    // 429 rate limiting is expected under spike — allow up to 80% rate-limited
    'rate_limited_rate':   ['rate<0.80'],
    // Our custom redirect latency metric
    'redirect_latency_ms': ['p(99)<200'],
  },
};

// ── Setup: seed a test URL and return its short_code ─────────────────────────
export function setup() {
  // Fast path: caller supplied a short code directly
  if (SHORT_CODE) {
    // Verify it actually resolves before starting the test
    const probe = http.get(`${BASE_URL}/${SHORT_CODE}`, { redirects: 0 });
    if (probe.status !== 307) {
      fail(
        `SHORT_CODE="${SHORT_CODE}" returned ${probe.status}, not 307. ` +
        `Ensure the code exists in the database.`
      );
    }
    console.log(`Using existing short code: ${SHORT_CODE}`);
    return { shortCode: SHORT_CODE, ownedUrlId: null, token: null };
  }

  // ── Step 1: Sign up a test user ───────────────────────────────────────────
  const uid      = Math.random().toString(36).substring(2, 9);
  const email    = `redir_${uid}@bittuly.test`;
  const username = `rd_${uid}`;
  const password = 'Redirect@123!';

  const signupRes = http.post(
    `${AUTH_URL}/api/auth/signup`,
    JSON.stringify({ username, email, password }),
    { headers: JSON_HEADERS },
  );
  if (signupRes.status !== 200) {
    fail(`[redirects setup] signup failed [${signupRes.status}]: ${signupRes.body}`);
  }
  const pendingToken = signupRes.json('pending_token');
  if (!pendingToken) fail('[redirects setup] no pending_token in signup response');

  // ── Step 2: Fetch OTP from debug store (MODE=development required) ────────
  const otpRes = http.get(`${AUTH_URL}/api/auth/debug/otp-store`);
  if (otpRes.status !== 200) {
    fail(
      `[redirects setup] debug/otp-store returned ${otpRes.status}. ` +
      `Is auth-service running with MODE=development? Or pass -e SHORT_CODE=<code>.`
    );
  }
  const entries = otpRes.json() || [];
  const entry   = entries.find((e) => e.email === email);
  if (!entry) fail(`[redirects setup] no OTP found for ${email}`);

  // ── Step 3: Verify OTP → get access_token ────────────────────────────────
  const verifyRes = http.post(
    `${AUTH_URL}/api/auth/verify-otp`,
    JSON.stringify({ pending_token: pendingToken, otp: entry.otp }),
    { headers: JSON_HEADERS },
  );
  if (verifyRes.status !== 201) {
    fail(`[redirects setup] verify-otp failed [${verifyRes.status}]: ${verifyRes.body}`);
  }
  const setCookie = verifyRes.headers['Set-Cookie'] || '';
  const match     = setCookie.match(/access_token=([^;]+)/);
  if (!match) fail('[redirects setup] no access_token cookie in verify-otp response');
  const token = match[1];

  // ── Step 4: Create a permanent short URL to use throughout the test ───────
  const targetUrl  = `https://example.com/redirect-load-test/${uid}`;
  const createRes  = http.post(
    `${BASE_URL}/api/urls`,
    JSON.stringify({ original_url: targetUrl }),
    { headers: { ...JSON_HEADERS, Cookie: `access_token=${token}` } },
  );
  if (createRes.status !== 201) {
    fail(`[redirects setup] create URL failed [${createRes.status}]: ${createRes.body}`);
  }
  const shortCode   = createRes.json('short_code');
  const ownedUrlId  = createRes.json('id');
  if (!shortCode) fail('[redirects setup] no short_code in create response');

  // ── Step 5: Warm up the cache (prime L1 + L2 before the ramp starts) ─────
  for (let i = 0; i < 5; i++) {
    http.get(`${BASE_URL}/${shortCode}`, { redirects: 0 });
  }

  console.log(`[redirects setup] Ready — short_code: ${shortCode} → ${targetUrl}`);
  return { shortCode, ownedUrlId, token };
}

// ── Teardown: remove the test URL ────────────────────────────────────────────
export function teardown(data) {
  if (!data.ownedUrlId || !data.token) return;
  const res = http.del(
    `${BASE_URL}/api/urls/${data.ownedUrlId}`,
    null,
    { headers: { ...JSON_HEADERS, Cookie: `access_token=${data.token}` } },
  );
  if (res.status === 204) {
    console.log(`[redirects teardown] Deleted URL id=${data.ownedUrlId}`);
  } else {
    console.warn(`[redirects teardown] Delete returned ${res.status}`);
  }
}

// ── Main VU loop ─────────────────────────────────────────────────────────────
export default function (data) {
  const res = http.get(`${BASE_URL}/${data.shortCode}`, {
    redirects: 0, // Assert the 307 directly — do NOT follow the redirect
    tags: { name: 'redirect' },
  });

  redirectLat.add(res.timings.duration);

  if (res.status === 307) {
    // ✅ Cache hit — successful redirect
    check(res, {
      'is HTTP 307':         (r) => r.status === 307,
      'has Location header': (r) => !!r.headers['Location'],
    });
    redirectHits.add(1);
    errorRate.add(0);
    rateLimitRate.add(0);

  } else if (res.status === 503 && res.body && res.body.includes('503 Service Temporarily Unavailable')) {
    // ⚠️  Rate limited by NGINX — expected under extreme spikes if limit is exceeded
    // NGINX ingress returns 503 HTML by default for rate limiting.
    rateLimitRate.add(1);
    errorRate.add(0); // Not a service logic error
    redirectMisses.add(1);
    // No console.warn — this is expected at extremely high VUs

  } else {
    // ❌ Actual error (500, 404, or actual 503 from backend JSON, etc.)
    redirectMisses.add(1);
    errorRate.add(1);
    rateLimitRate.add(0);
    console.warn(`Unexpected response: ${res.status} ${res.url}`);
  }

  // 10ms pacing — realistic browser-like pacing
  sleep(0.01);
}
