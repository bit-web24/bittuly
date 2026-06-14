import { AlertTriangle } from "lucide-react"

export function Unavailable() {
  return (
    <div className="flex min-h-svh items-center justify-center bg-background px-4">
      <div className="w-full max-w-md page-enter">
        <div className="flex flex-col items-center rounded-xl border bg-card p-8 text-center shadow-sm">
          <div className="mb-6 flex h-16 w-16 items-center justify-center rounded-full bg-destructive/10">
            <AlertTriangle className="h-8 w-8 text-destructive" />
          </div>
          
          <h1 className="mb-2 text-2xl font-bold tracking-tight">Link Unavailable</h1>
          
          <p className="mb-8 text-muted-foreground">
            The short link you clicked does not exist or has been deleted by its owner.
          </p>
          
        </div>
      </div>
    </div>
  )
}
