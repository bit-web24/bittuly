const BASE_URL = "http://localhost:3000"

export interface ApiError {
  status: number
  data: Record<string, unknown>
}

export async function apiRequest<T = unknown>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, {
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
