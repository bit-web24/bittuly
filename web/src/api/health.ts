const BASE_URL = "http://localhost:3000"

export interface HealthData {
  status: "healthy" | "degraded"
  postgres: string
  redis: string
  version: string
  uptime_secs: number
}

/**
 * Fetches /health — never throws.
 * Returns the response body whether the server returned 200 or 503.
 */
export async function getHealth(): Promise<{ data: HealthData; ok: boolean }> {
  try {
    const res = await fetch(`${BASE_URL}/health`)
    const data: HealthData = await res.json()
    return { data, ok: res.ok }
  } catch {
    // Network-level failure (server unreachable)
    return {
      ok: false,
      data: {
        status: "degraded",
        postgres: "error: unreachable",
        redis: "error: unreachable",
        version: "—",
        uptime_secs: 0,
      },
    }
  }
}
