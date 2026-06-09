import * as React from "react"
import { Copy, Check, Trash2, ExternalLink } from "lucide-react"
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
}

export function UrlItem({ url, isNew = false }: UrlItemProps) {
  const [copied, setCopied] = React.useState(false)
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

        {/* Actions */}
        <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity duration-100 group-hover:opacity-100">
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

          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                disabled
                aria-label="Delete URL"
              >
                <Trash2 className="size-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>URL deletion coming soon</TooltipContent>
          </Tooltip>
        </div>
      </div>
    </TooltipProvider>
  )
}
