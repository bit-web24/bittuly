/**
 * tests/load/auth.js
 *
 * Load test: auth-service endpoints (login + token refresh path)
 *
 * Tests the login endpoint under sustained concurrent load to verify:
 *   - bcrypt verification latency stays within SLO
 *   - JWT cookie issuance works correctly under concurrency
 *   - Rate limiting (5 req/min per IP) kicks in as expected
 *
 * Prerequisites:
 *   A real user must exist. Provide credentials via env vars:
 *     EMAIL    — registered user email
 *     PASSWORD — user password
 *
 * Run:
 *   k6 run -e EMAIL=user@example.com -e PASSWORD=yourpassword tests/load/auth.js
 *
 * Note: The auth rate limit (5 req/min on login/signup) will trigger 429s
 * intentionally at higher VU counts. The thresholds account for this by
 * checking the 429 rate separately rather than treating them as failures.
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

// ── Custom metrics ──────────────────────────────────────────────────────────
const loginSuccess  = new Counter('login_success');
const loginRateLim  = new Counter('login_rate_limited');
const loginFailed   = new Counter('login_failed');
const loginLat      = new Trend('login_latency_ms', true);
const rateLimitRate = new Rate('rate_limit_rate');

// ── Config ──────────────────────────────────────────────────────────────────
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8000';
const EMAIL    = __ENV.EMAIL    || '';
const PASSWORD = __ENV.PASSWORD || '';

const JSON_HEADERS = { 'Content-Type': 'application/json' };

// ── Load profile — kept LOW intentionally to respect rate limiting ───────────
export const options = {
  stages: [
    { duration: '15s', target: 5  }, // Gentle ramp
    { duration: '45s', target: 10 }, // Sustained (rate limit zone)
    { duration: '15s', target: 20 }, // Spike to observe 429s
    { duration: '15s', target: 0  }, // Ramp-down
  ],
  thresholds: {
    // bcrypt is intentionally slow (~250ms per hash at cost 12).
    // Under load, a local machine's CPU will saturate and queue requests.
    'http_req_duration':   ['p(99)<2000', 'p(50)<500'],
    // We ONLY count non-429 non-200 responses as failures
    'login_failed':        ['count<5'],
    // Rate limiting is expected behaviour, not a failure
    'rate_limit_rate':     ['rate<0.5'],
    'login_latency_ms':    ['p(99)<2000'],
  },
};

// ── Setup: self-provision a test user — no pre-existing state required ────────
export function setup() {
  // ── Fast path: pre-existing credentials provided via env vars ────────────
  if (EMAIL && PASSWORD) {
    console.log(`Using provided credentials for ${EMAIL}. Skipping signup.`);
    return { email: EMAIL, password: PASSWORD };
  }

  // ── Self-provision: create a fresh test user via OTP flow ─────────────────
  // Requires MODE=development on auth-service so OTPs are stored in memory
  // and exposed at GET /api/auth/debug/otp-store.
  const uid      = Math.random().toString(36).substring(2, 9);
  const email    = `authtest_${uid}@bittuly.test`;
  const username = `at_${uid}`;
  const password = 'AuthTest@123!';

  // Step 1 — Signup → OTP stored in memory (no real email sent)
  const signupRes = http.post(
    `${BASE_URL}/api/auth/signup`,
    JSON.stringify({ username, email, password }),
    { headers: JSON_HEADERS },
  );

  if (signupRes.status !== 200) {
    fail(`[auth.js setup] Signup failed [${signupRes.status}]: ${signupRes.body}`);
  }

  const pendingToken = signupRes.json('pending_token');
  if (!pendingToken) {
    fail('[auth.js setup] Signup did not return a pending_token.');
  }

  // Step 2 — Fetch OTP from in-memory debug store
  const otpStoreRes = http.get(`${BASE_URL}/api/auth/debug/otp-store`);

  if (otpStoreRes.status === 404) {
    fail(
      '[auth.js setup] GET /api/auth/debug/otp-store returned 404. ' +
      'Ensure auth-service is running with MODE=development, ' +
      'or pass -e EMAIL=... -e PASSWORD=... to use existing credentials.'
    );
  }
  if (otpStoreRes.status !== 200) {
    fail(`[auth.js setup] OTP store fetch failed [${otpStoreRes.status}]: ${otpStoreRes.body}`);
  }

  const entries = otpStoreRes.json();
  const entry   = (entries || []).find((e) => e.email === email);
  if (!entry || !entry.otp) {
    fail(`[auth.js setup] No OTP found for ${email} in debug store.`);
  }

  // Step 3 — Verify OTP → create user in DB
  const verifyRes = http.post(
    `${BASE_URL}/api/auth/verify-otp`,
    JSON.stringify({ pending_token: pendingToken, otp: entry.otp }),
    { headers: JSON_HEADERS },
  );

  if (verifyRes.status !== 201) {
    fail(`[auth.js setup] verify-otp failed [${verifyRes.status}]: ${verifyRes.body}`);
  }

  console.log(`[auth.js setup] Test user provisioned: ${email}`);

  // Return plain credentials — the load test drives repeated logins using these
  return { email, password };
}

// ── Main VU loop ──────────────────────────────────────────────────────────────
export default function (data) {
  if (!data.email) { sleep(1); return; }

  const res = http.post(
    `${BASE_URL}/api/auth/login`,
    JSON.stringify({ email: data.email, password: data.password }),
    { headers: JSON_HEADERS, tags: { name: 'login' } },
  );

  loginLat.add(res.timings.duration);

  if (res.status === 200) {
    loginSuccess.add(1);
    rateLimitRate.add(0);

    check(res, {
      'login: has access_token cookie':  (r) => r.headers['Set-Cookie']?.includes('access_token') ?? false,
      'login: has refresh_token cookie': (r) => r.headers['Set-Cookie']?.includes('refresh_token') ?? false,
    });

  } else if (res.status === 429) {
    // Expected under high load due to NGINX rate limit (5 req/min on /login)
    loginRateLim.add(1);
    rateLimitRate.add(1);
    // Back off to respect the rate limit window
    sleep(12);

  } else if (res.status === 401) {
    // Wrong credentials — not a system error, just log it
    loginFailed.add(1);
    rateLimitRate.add(0);
    console.warn(`Login returned 401 — check EMAIL/PASSWORD env vars.`);

  } else {
    loginFailed.add(1);
    rateLimitRate.add(0);
    console.error(`Unexpected login response [${res.status}]: ${res.body}`);
  }

  // Realistic pause between login attempts (users don't spam login)
  sleep(1);
}
