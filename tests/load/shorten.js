/**
 * tests/load/shorten.js
 *
 * Load test: URL shortening write path  (POST /api/urls)
 *
 * Strategy
 * ─────────
 * The auth-service uses OTP-based signup. In development mode (MODE=development)
 * no real email is sent — the OTP is stored in an in-memory cache exposed at:
 *
 *   GET /api/auth/debug/otp-store   → [{ email, otp, created_at }, ...]
 *
 * This script uses that endpoint to complete the OTP flow automatically.
 * No manual copy-pasting of tokens or OTPs is needed.
 *
 * Quick start (fully automatic — requires MODE=development on auth-service):
 * ──────────────────────────────────────────────────────────────────────────
 *   k6 run tests/load/shorten.js
 *
 * Quick start with a pre-issued JWT (skips signup entirely):
 * ──────────────────────────────────────────────────────────
 *   k6 run -e TOKEN=<access_token_cookie_value> tests/load/shorten.js
 *
 * Run inside kind cluster:
 *   kubectl run k6 --rm -i --restart=Never --image=grafana/k6 -- run \
 *     -e BASE_URL=http://url-service.bittuly.svc.cluster.local:3002 \
 *     -e AUTH_URL=http://auth-service.bittuly.svc.cluster.local:3001 \
 *     - < tests/load/shorten.js
 */

import http from 'k6/http';
import { check, sleep, fail } from 'k6';
import { Counter, Trend, Rate } from 'k6/metrics';

// ── Custom metrics ──────────────────────────────────────────────────────────
const urlsCreated  = new Counter('urls_created');
const urlsDeleted  = new Counter('urls_deleted');
const shortenLat   = new Trend('shorten_latency_ms', true);
const authFailRate = new Rate('auth_failure_rate');

// ── Config ──────────────────────────────────────────────────────────────────
const BASE_URL = __ENV.BASE_URL  || 'http://localhost:8000';
const AUTH_URL = __ENV.AUTH_URL  || BASE_URL;
const TOKEN    = __ENV.TOKEN     || '';   // Pre-issued JWT — bypass OTP entirely
const OTP      = __ENV.OTP      || '';   // 6-digit OTP from auth-service logs

const JSON_HEADERS = { 'Content-Type': 'application/json' };

// ── Load profile ─────────────────────────────────────────────────────────────
export const options = {
  stages: [
    { duration: '20s', target: 10  }, // Warm-up
    { duration: '40s', target: 50  }, // Sustained write load
    { duration: '20s', target: 100 }, // Spike
    { duration: '30s', target: 50  }, // Recovery
    { duration: '10s', target: 0   }, // Ramp-down
  ],
  thresholds: {
    // SLO: shorten p99 < 200ms, p50 < 50ms (write path, DB involved)
    'http_req_duration':  ['p(99)<200', 'p(50)<50'],
    // SLO: 99.5% availability
    'http_req_failed':    ['rate<0.005'],
    'auth_failure_rate':  ['rate<0.01'],
    'shorten_latency_ms': ['p(99)<200'],
  },
};

// ── Setup: authenticate once, share token across all VUs ─────────────────────
export function setup() {
  // ── Path A: pre-issued JWT provided — skip signup entirely ───────────────
  if (TOKEN) {
    console.log('Using pre-issued TOKEN from env var. Skipping signup.');
    return { token: TOKEN };
  }

  // ── Path B: automatic signup via in-memory OTP store (MODE=development) ──
  // Requires MODE=development on auth-service so OTPs are stored in memory
  // and exposed at GET /api/auth/debug/otp-store.
  const uid      = Math.random().toString(36).substring(2, 9);
  const email    = `loadtest_${uid}@bittuly.test`;
  const username = `lt_${uid}`;
  const password = 'LoadTest@123!';

  // Step 1 — Request signup → auth-service stores OTP in memory, no real email
  const signupRes = http.post(
    `${AUTH_URL}/api/auth/signup`,
    JSON.stringify({ username, email, password }),
    { headers: JSON_HEADERS },
  );

  if (signupRes.status !== 200) {
    fail(`Signup failed [${signupRes.status}]: ${signupRes.body}`);
  }

  const pendingToken = signupRes.json('pending_token');
  if (!pendingToken) {
    fail('Signup did not return a pending_token.');
  }

  // Step 2 — Fetch OTP from debug endpoint (only available in MODE=development)
  const otpStoreRes = http.get(`${AUTH_URL}/api/auth/debug/otp-store`);

  if (otpStoreRes.status === 404) {
    fail(
      'GET /api/auth/debug/otp-store returned 404. ' +
      'Ensure auth-service is running with MODE=development.'
    );
  }
  if (otpStoreRes.status !== 200) {
    fail(`Failed to fetch OTP store [${otpStoreRes.status}]: ${otpStoreRes.body}`);
  }

  const otpEntries = otpStoreRes.json();
  if (!Array.isArray(otpEntries) || otpEntries.length === 0) {
    fail('OTP store is empty. Signup may not have stored the OTP correctly.');
  }

  // Find OTP entry for the email we just signed up with
  const entry = otpEntries.find((e) => e.email === email);
  if (!entry || !entry.otp) {
    fail(`No OTP found for ${email} in debug store.`);
  }

  const otp = entry.otp;
  console.log(`Signup OK — email: ${email}, OTP retrieved from debug store.`);

  // Step 3 — Verify OTP → receive access_token + refresh_token cookies
  const verifyRes = http.post(
    `${AUTH_URL}/api/auth/verify-otp`,
    JSON.stringify({ pending_token: pendingToken, otp }),
    { headers: JSON_HEADERS },
  );

  if (verifyRes.status !== 201) {
    fail(`OTP verification failed [${verifyRes.status}]: ${verifyRes.body}`);
  }

  // Extract access_token value from Set-Cookie header
  const setCookie = verifyRes.headers['Set-Cookie'] || '';
  const match     = setCookie.match(/access_token=([^;]+)/);
  if (!match) {
    fail('No access_token cookie in verify-otp response.');
  }

  const token = match[1];
  console.log(`Auth OK — JWT obtained for ${email}. Starting load test...`);
  return { token, email };
}

// ── Teardown: nothing to clean up (URLs deleted per-VU below) ───────────────
export function teardown(data) {
  console.log(`Load test complete. urls_created counter includes all VU iterations.`);
}

// ── Main VU loop ─────────────────────────────────────────────────────────────
export default function (data) {
  if (!data || !data.token) {
    authFailRate.add(1);
    console.error('No auth token available — skipping VU iteration.');
    sleep(1);
    return;
  }

  authFailRate.add(0);

  const cookieJar = http.cookieJar();
  cookieJar.set(BASE_URL, 'access_token', data.token);

  const headers = {
    ...JSON_HEADERS,
    'Cookie': `access_token=${data.token}`,
  };

  // ── Step 1: Shorten a unique URL ─────────────────────────────────────────
  const targetUrl  = `https://example.com/load-test/${__VU}-${__ITER}-${Date.now()}`;
  const shortenRes = http.post(
    `${BASE_URL}/api/urls`,
    JSON.stringify({ original_url: targetUrl }),
    { headers, tags: { name: 'shorten' } },
  );

  const shortenOk = check(shortenRes, {
    'shorten: status 201':      (r) => r.status === 201,
    'shorten: has short_code':  (r) => {
      try { return r.json('short_code') !== undefined; } catch { return false; }
    },
    'shorten: has id':          (r) => {
      try { return r.json('id') !== undefined; } catch { return false; }
    },
  });

  shortenLat.add(shortenRes.timings.duration);

  if (!shortenOk) {
    console.warn(`Shorten failed [${shortenRes.status}]: ${shortenRes.body}`);
    sleep(0.5);
    return;
  }

  urlsCreated.add(1);

  const urlId    = shortenRes.json('id');
  const code     = shortenRes.json('short_code');

  // ── Step 2: Verify the redirect resolves correctly ───────────────────────
  const redirectRes = http.get(`${BASE_URL}/${code}`, {
    redirects: 0,
    tags: { name: 'verify_redirect' },
  });

  check(redirectRes, {
    'redirect: status 307':           (r) => r.status === 307,
    'redirect: Location matches URL': (r) => r.headers['Location'] === targetUrl,
  });

  // ── Step 3: Delete the URL to keep DB clean across test runs ─────────────
  const deleteRes = http.del(
    `${BASE_URL}/api/urls/${urlId}`,
    null,
    { headers, tags: { name: 'delete_url' } },
  );

  check(deleteRes, {
    'delete: status 204': (r) => r.status === 204,
  });

  if (deleteRes.status === 204) {
    urlsDeleted.add(1);
  }

  sleep(0.1);
}
