import * as React from "react"
import { Copy, Check, Trash2, ExternalLink, X, BarChart2 } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import type { ShortenedUrl } from "@/api/urls"

const BASE_URL = "http://localhost:3000"

interface UrlItemProps {
  url: ShortenedUrl
  isNew?: boolean
  onDelete?: (id: number) => void
}

export function UrlItem({ url, isNew = false, onDelete }: UrlItemProps) {
  const [copied, setCopied] = React.useState(false)
  const [confirming, setConfirming] = React.useState(false)
  const confirmTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null)
  const shortUrl = `${BASE_URL}/${url.short_code}`

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(shortUrl)
      setCopied(true)
      toast.success("Copied to clipboard!")
      setTimeout(() => setCopied(false), 2000)
    } catch {
      toast.error("Failed to copy")
    }
  }

  const handleDeleteClick = () => {
    if (!confirming) {
      // First click — enter confirm mode, auto-reset after 3s
      setConfirming(true)
      confirmTimerRef.current = setTimeout(() => setConfirming(false), 3000)
    } else {
      // Second click — execute
      if (confirmTimerRef.current) clearTimeout(confirmTimerRef.current)
      setConfirming(false)
      onDelete?.(url.id)
    }
  }

  const handleCancelDelete = (e: React.MouseEvent) => {
    e.stopPropagation()
    if (confirmTimerRef.current) clearTimeout(confirmTimerRef.current)
    setConfirming(false)
  }

  // Cleanup timer on unmount
  React.useEffect(() => {
    return () => {
      if (confirmTimerRef.current) clearTimeout(confirmTimerRef.current)
    }
  }, [])

  return (
    <TooltipProvider>
      <div
        className={`group flex items-center gap-4 rounded-lg border bg-card px-4 py-3 transition-colors duration-150 hover:bg-accent ${
          isNew ? "animate-in slide-in-from-top-2 fade-in duration-200" : ""
        }`}
      >
        {/* Short code link */}
        <a
          href={shortUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="flex min-w-0 shrink-0 items-center gap-1.5 text-sm font-medium text-notion-blue hover:underline"
        >
          <span className="truncate max-w-[140px]">{url.short_code}</span>
          <ExternalLink className="size-3 opacity-60" />
        </a>

        {/* Divider */}
        <div className="h-4 w-px shrink-0 bg-border" />

        {/* Original URL */}
        <Tooltip>
          <TooltipTrigger asChild>
            <span className="min-w-0 flex-1 truncate text-sm text-muted-foreground">
              {url.original_url}
            </span>
          </TooltipTrigger>
          <TooltipContent side="top" className="max-w-xs break-all">
            {url.original_url}
          </TooltipContent>
        </Tooltip>

        {/* Click count */}
        <div className="flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground mr-2">
          <BarChart2 className="size-3.5" />
          <span>{url.click_count} {url.click_count === 1 ? "click" : "clicks"}</span>
        </div>

        {/* Actions */}
        <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity duration-100 group-hover:opacity-100">
          {/* Copy */}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={handleCopy}
                aria-label="Copy short URL"
              >
                {copied ? (
                  <Check className="size-3.5 text-green-600" />
                ) : (
                  <Copy className="size-3.5" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent>{copied ? "Copied!" : "Copy link"}</TooltipContent>
          </Tooltip>

          {/* Delete — two-step inline confirm */}
          {confirming ? (
            <>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={handleDeleteClick}
                aria-label="Confirm delete"
                className="text-destructive hover:bg-destructive/10 hover:text-destructive"
              >
                <Trash2 className="size-3.5" />
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={handleCancelDelete}
                aria-label="Cancel delete"
              >
                <X className="size-3.5" />
              </Button>
            </>
          ) : (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={handleDeleteClick}
                  aria-label="Delete URL"
                >
                  <Trash2 className="size-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Delete</TooltipContent>
            </Tooltip>
          )}
        </div>
      </div>
    </TooltipProvider>
  )
}

