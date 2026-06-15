import * as React from "react"
import { Copy, Check, Trash2, ExternalLink, BarChart2, X, Timer } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import type { ShortenedUrl } from "@/api/urls"
import { URLS_BASE_URL } from "@/api/client"

interface UrlItemProps {
  url: ShortenedUrl
  isNew?: boolean
  onDelete?: (id: number) => void
}

export function UrlItem({ url, isNew = false, onDelete }: UrlItemProps) {
  const [copied, setCopied] = React.useState(false)
  const [deleteOpen, setDeleteOpen] = React.useState(false)
  const [detailsOpen, setDetailsOpen] = React.useState(false)
  const shortUrl = `${URLS_BASE_URL}/${url.short_code}`

  const isExpired = url.expires_at ? new Date(url.expires_at) < new Date() : false
  const formattedExpiry = url.expires_at 
    ? new Date(url.expires_at).toLocaleString([], { dateStyle: 'short', timeStyle: 'short' })
    : null

  const handleCopy = (e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()

    const executeCopy = () => {
      if (navigator.clipboard && window.isSecureContext) {
        return navigator.clipboard.writeText(shortUrl)
      } else {
        // Fallback for insecure HTTP contexts
        const textArea = document.createElement("textarea")
        textArea.value = shortUrl
        textArea.style.position = "fixed" // Prevent scrolling to bottom
        textArea.style.left = "-999999px"
        textArea.style.top = "0"
        document.body.appendChild(textArea)
        textArea.focus()
        textArea.select()
        try {
          const successful = document.execCommand("copy")
          if (!successful) throw new Error("copy command failed")
          return Promise.resolve()
        } catch (error) {
          console.error("Fallback copy error:", error)
          return Promise.reject(error)
        } finally {
          textArea.remove()
        }
      }
    }

    executeCopy()
      .then(() => {
        setCopied(true)
        toast.success("Copied to clipboard!")
        setTimeout(() => setCopied(false), 2000)
      })
      .catch(() => {
        toast.error("Failed to copy shortcode")
      })
  }



  return (
    <Dialog open={detailsOpen} onOpenChange={setDetailsOpen}>
      <TooltipProvider>
        <div
          onClick={() => setDetailsOpen(true)}
          className={`group flex items-center gap-4 rounded-lg border bg-card px-4 py-3 transition-colors duration-150 hover:bg-accent cursor-pointer ${
            isNew ? "animate-in slide-in-from-top-2 fade-in duration-200" : ""
          }`}
        >
        {/* Short code link */}
        <a
          href={shortUrl}
          target="_blank"
          rel="noopener noreferrer"
          onClick={(e) => e.stopPropagation()}
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

        {/* Expiry */}
        {formattedExpiry && (
          <div className={`flex shrink-0 items-center gap-1.5 text-xs mr-2 ${isExpired ? "text-destructive font-medium" : "text-muted-foreground"}`}>
            <Timer className="size-3.5" />
            <span>{isExpired ? "Expired" : formattedExpiry}</span>
          </div>
        )}

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

          {/* Delete — Modal confirmation */}
          <AlertDialog open={deleteOpen} onOpenChange={setDeleteOpen}>
            <Tooltip>
              <TooltipTrigger asChild>
                <AlertDialogTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label="Delete URL"
                    onClick={(e) => e.stopPropagation()}
                    className="hover:bg-destructive/10 hover:text-destructive"
                  >
                    <Trash2 className="size-3.5" />
                  </Button>
                </AlertDialogTrigger>
              </TooltipTrigger>
              <TooltipContent>Delete</TooltipContent>
            </Tooltip>

            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Delete this short link?</AlertDialogTitle>
                <AlertDialogDescription>
                  This will permanently delete the shortened link (<strong>bittuly.com/{url.short_code}</strong>) and remove all its click analytics. This action cannot be undone.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => onDelete?.(url.id)}
                  className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                >
                  Delete Link
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>
      </div>

      <DialogContent className="sm:max-w-xl" showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Link Details</DialogTitle>
        </DialogHeader>
        <div className="space-y-6 py-4">
          <div className="space-y-1.5">
            <span className="text-sm font-medium text-muted-foreground">Original URL</span>
            <p className="text-sm break-all bg-muted/50 p-3 rounded-md">{url.original_url}</p>
          </div>
          <div className="space-y-1.5">
            <span className="text-sm font-medium text-muted-foreground">Short Link</span>
            <div className="flex items-center gap-3">
              <a href={shortUrl} target="_blank" rel="noopener noreferrer" className="text-sm font-medium text-notion-blue hover:underline">
                {shortUrl}
              </a>
              <Button variant="outline" size="sm" className="h-7 text-xs" onClick={handleCopy}>
                {copied ? <Check className="size-3 mr-1 text-green-600" /> : <Copy className="size-3 mr-1" />}
                {copied ? "Copied" : "Copy"}
              </Button>
            </div>
          </div>
          <div className="grid grid-cols-2 gap-4 border-t pt-4">
            <div className="space-y-1">
              <span className="text-sm font-medium text-muted-foreground">Total Clicks</span>
              <p className="text-2xl font-semibold">{url.click_count}</p>
            </div>
            <div className="space-y-1">
              <span className="text-sm font-medium text-muted-foreground">Created on</span>
              <p className="text-base">{new Date(url.created_at).toLocaleDateString([], { dateStyle: 'medium' })}</p>
            </div>
          </div>
          {formattedExpiry && (
            <div className="space-y-1 border-t pt-4">
              <span className="text-sm font-medium text-muted-foreground">Expiration Status</span>
              <div className="flex items-center gap-2 mt-1">
                <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-semibold ${isExpired ? "bg-destructive/10 text-destructive" : "bg-green-100 text-green-800"}`}>
                  {isExpired ? "Expired" : "Active"}
                </span>
                <span className="text-sm text-muted-foreground">
                  (Expires: {formattedExpiry})
                </span>
              </div>
            </div>
          )}
        </div>
        
        {/* Footer with Delete Action */}
        <div className="flex items-center justify-between border-t pt-4">
          <Button variant="outline" onClick={() => setDetailsOpen(false)}>
            Close
          </Button>
          <Button 
            variant="destructive" 
            onClick={() => setDeleteOpen(true)}
            className="bg-destructive/90 hover:bg-destructive"
          >
            <Trash2 className="size-4 mr-2" />
            Delete Link
          </Button>
        </div>
      </DialogContent>
    </TooltipProvider>
  </Dialog>
  )
}

