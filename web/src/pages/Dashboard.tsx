import * as React from "react"
import { toast } from "sonner"
import { Link2, Plus } from "lucide-react"
import { createUrl, deleteUrl, getUrls, type ShortenedUrl } from "@/api/urls"
import { AppLayout } from "@/components/AppLayout"
import { UrlItem } from "@/components/UrlItem"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Spinner } from "@/components/ui/spinner"

function isValidUrl(url: string): boolean {
  try {
    const u = new URL(url)
    return u.protocol === "http:" || u.protocol === "https:"
  } catch {
    return false
  }
}

export function Dashboard() {
  const [urls, setUrls] = React.useState<ShortenedUrl[]>([])
  const [newIds, setNewIds] = React.useState<Set<string>>(new Set())
  const [inputUrl, setInputUrl] = React.useState("")
  const [inputError, setInputError] = React.useState(false)
  const [isShortening, setIsShortening] = React.useState(false)
  const [isLoadingUrls, setIsLoadingUrls] = React.useState(true)
  const inputRef = React.useRef<HTMLInputElement>(null)

  // Load URLs on mount
  React.useEffect(() => {
    getUrls()
      .then((data) => setUrls(data))
      .catch(() => toast.error("Failed to load URLs."))
      .finally(() => setIsLoadingUrls(false))
  }, [])

  const handleShorten = async (e: React.FormEvent) => {
    e.preventDefault()
    const trimmed = inputUrl.trim()
    if (!trimmed || !isValidUrl(trimmed)) {
      setInputError(true)
      inputRef.current?.focus()
      return
    }
    setInputError(false)
    setIsShortening(true)
    try {
      const created = await createUrl(trimmed)
      setUrls((prev) => [created, ...prev])
      setNewIds((prev) => new Set(prev).add(created.id))
      setInputUrl("")
      setTimeout(() => {
        setNewIds((prev) => {
          const next = new Set(prev)
          next.delete(created.id)
          return next
        })
      }, 1000)
    } catch {
      toast.error("Something went wrong. Please try again.")
    } finally {
      setIsShortening(false)
    }
  }

  const handleDelete = async (id: number) => {
    // Optimistic removal
    setUrls((prev) => prev.filter((u) => u.id !== id))
    try {
      await deleteUrl(id)
      toast.success("Link deleted.")
    } catch {
      // Revert on failure
      toast.error("Failed to delete link. Please try again.")
      getUrls()
        .then(setUrls)
        .catch(() => {})
    }
  }

  return (
    <AppLayout title="My Links">
      <div className="mx-auto max-w-3xl space-y-6">
        {/* Input card */}
        <div className="rounded-xl border bg-card p-5 shadow-sm">
          <p className="mb-3 text-sm font-medium text-muted-foreground">
            Shorten a new URL
          </p>
          <form onSubmit={handleShorten} className="flex gap-2">
            <div className="flex-1">
              <Input
                ref={inputRef}
                placeholder="Paste a long URL here..."
                value={inputUrl}
                onChange={(e) => {
                  setInputUrl(e.target.value)
                  if (inputError) setInputError(false)
                }}
                aria-invalid={inputError}
                className={inputError ? "animate-shake" : ""}
              />
              {inputError && (
                <p className="mt-1 text-xs text-destructive">
                  Please enter a valid URL.
                </p>
              )}
            </div>
            <Button type="submit" disabled={isShortening} className="shrink-0">
              {isShortening ? (
                <Spinner className="size-4" />
              ) : (
                <>
                  <Plus className="size-4" />
                  Shorten
                </>
              )}
            </Button>
          </form>
        </div>

        {/* URL list */}
        <div>
          <div className="mb-3 flex items-center gap-2">
            <h2 className="text-sm font-semibold">Your links</h2>
            {!isLoadingUrls && (
              <Badge variant="secondary">{urls.length}</Badge>
            )}
          </div>

          {isLoadingUrls ? (
            <div className="flex justify-center py-12">
              <Spinner className="size-5 text-muted-foreground" />
            </div>
          ) : urls.length === 0 ? (
            <div className="flex flex-col items-center gap-3 rounded-xl border border-dashed py-16 text-center">
              <Link2 className="size-8 text-muted-foreground opacity-40" />
              <div>
                <p className="text-sm font-medium">No links yet.</p>
                <p className="mt-0.5 text-sm text-muted-foreground">
                  Paste a URL above to get started.
                </p>
              </div>
            </div>
          ) : (
            <div className="space-y-2">
              {urls.map((url) => (
                <UrlItem
                  key={url.id}
                  url={url}
                  isNew={newIds.has(url.id)}
                  onDelete={handleDelete}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </AppLayout>
  )
}
