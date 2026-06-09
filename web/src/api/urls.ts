import { apiRequest } from "./client"

export interface ShortenedUrl {
  id: string
  short_code: string
  original_url: string
  created_at: string
}

export async function createUrl(original_url: string): Promise<ShortenedUrl> {
  return apiRequest("/", {
    method: "POST",
    body: JSON.stringify({ original_url }),
  })
}

export async function getUrls(): Promise<ShortenedUrl[]> {
  return apiRequest("/")
}
