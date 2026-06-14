export const AUTH_BASE_URL = import.meta.env.VITE_AUTH_API_URL || "http://localhost:3001"
export const URLS_BASE_URL = import.meta.env.VITE_URLS_API_URL || "http://localhost:3002"

export interface ApiError {
  status: number
  data: Record<string, unknown>
}

export async function apiRequest<T = unknown>(
  baseUrl: string,
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const res = await fetch(`${baseUrl}${path}`, {
    ...options,
    credentials: "include",
    headers: {
      "Content-Type": "application/json",
      ...options.headers,
    },
  })

  if (res.status === 204) return null as T

  const data = await res.json().catch(() => ({}))

  if (!res.ok) {
    const err: ApiError = { status: res.status, data }
    throw err
  }

  return data as T
}
