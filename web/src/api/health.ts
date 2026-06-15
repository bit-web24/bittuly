const URLS_BASE_URL = import.meta.env.VITE_URLS_API_URL || "http://localhost:8000"

export interface HealthData {
  status: "healthy" | "degraded"
  postgres: string
  redis: string
  version: string
  uptime_secs: number
}

/**
 * Fetches /api/urls/health — never throws.
 * Returns the response body whether the server returned 200 or 503.
 */
export async function getHealth(): Promise<{ data: HealthData; ok: boolean }> {
  try {
    const res = await fetch(`${URLS_BASE_URL}/api/urls/health`)
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
