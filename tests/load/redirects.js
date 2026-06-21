import http from 'k6/http';
import { check, sleep } from 'k6';

// Read configuration from environment variables or use defaults
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8000';
const SHORT_CODE = __ENV.SHORT_CODE || 'github'; // Fallback to a hardcoded code

export const options = {
  // A standard load test configuration
  stages: [
    { duration: '30s', target: 100 }, // Ramp-up to 100 users over 30s
    { duration: '1m', target: 100 },  // Stay at 100 users for 1 minute
    { duration: '30s', target: 0 },   // Ramp-down to 0 users
  ],
  thresholds: {
    // SLO constraints from TARGET.md
    'http_req_duration': ['p(99)<15', 'p(50)<5'], // p99 < 15ms, p50 < 5ms for cache hits
    'http_req_failed': ['rate<0.001'],            // < 0.1% failure rate (99.9% availability)
  },
};

export default function () {
  // Hit the redirect endpoint. The service should return a 307 redirect.
  // We don't want k6 to follow the redirect automatically, so we specify redirects: 0
  const res = http.get(`${BASE_URL}/${SHORT_CODE}`, { redirects: 0 });

  check(res, {
    'is status 307': (r) => r.status === 307,
    'has location header': (r) => r.headers['Location'] !== undefined,
  });

  // Short sleep to simulate real user behavior and prevent overwhelming network sockets instantly
  sleep(0.01);
}
