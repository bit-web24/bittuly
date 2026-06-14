import { apiRequest, URLS_BASE_URL } from "./client"

export interface ShortenedUrl {
  id: number
  short_code: string
  original_url: string
  click_count: number
  created_at: string
}

export interface UrlsPage {
  urls: ShortenedUrl[]
  next_cursor: string | null
}

export async function createUrl(original_url: string): Promise<ShortenedUrl> {
  return apiRequest(URLS_BASE_URL, "/api/urls", {
    method: "POST",
    body: JSON.stringify({ original_url }),
  })
}

export async function getUrlsPage(
  cursor?: string | null,
  limit = 20,
  search?: string
): Promise<UrlsPage> {
  const params = new URLSearchParams()
  if (cursor) params.set("cursor", cursor)
  params.set("limit", String(limit))
  if (search && search.trim()) params.set("search", search.trim())
  return apiRequest(URLS_BASE_URL, `/api/urls?${params.toString()}`)
}

/** Convenience: fetch all URLs for non-paginated views (Insights). */
export async function getUrls(): Promise<ShortenedUrl[]> {
  const params = new URLSearchParams({ limit: "100" })
  const page = await apiRequest<UrlsPage>(URLS_BASE_URL, `/api/urls?${params.toString()}`)
  return page.urls
}

export async function deleteUrl(id: number): Promise<null> {
  return apiRequest(URLS_BASE_URL, `/api/urls/${id}`, { method: "DELETE" })
}
