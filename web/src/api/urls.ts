import { apiRequest } from "./client"

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
  return apiRequest("/", {
    method: "POST",
    body: JSON.stringify({ original_url }),
  })
}

export async function getUrlsPage(
  cursor?: string | null,
  limit = 20
): Promise<UrlsPage> {
  const params = new URLSearchParams()
  if (cursor) params.set("cursor", cursor)
  params.set("limit", String(limit))
  return apiRequest(`/?${params.toString()}`)
}

/** Convenience: fetch all URLs for non-paginated views (Insights). */
export async function getUrls(): Promise<ShortenedUrl[]> {
  const params = new URLSearchParams({ limit: "100" })
  const page = await apiRequest<UrlsPage>(`/?${params.toString()}`)
  return page.urls
}

export async function deleteUrl(id: number): Promise<null> {
  return apiRequest(`/${id}`, { method: "DELETE" })
}
