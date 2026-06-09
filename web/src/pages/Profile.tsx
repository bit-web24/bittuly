import * as React from "react"
import { format } from "date-fns"
import { Pencil, Check, X } from "lucide-react"
import { toast } from "sonner"
import { updateUser } from "@/api/auth"
import { getUrls } from "@/api/urls"
import { useAuth, USER_ID_KEY } from "@/context/AuthContext"
import { AppLayout } from "@/components/AppLayout"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Spinner } from "@/components/ui/spinner"

function getInitials(name: string): string {
  return name
    .split(" ")
    .map((n) => n[0])
    .join("")
    .toUpperCase()
    .slice(0, 2)
}

export function Profile() {
  const { user, setUser } = useAuth()
  const [urlCount, setUrlCount] = React.useState<number | null>(null)
  const [editing, setEditing] = React.useState(false)
  const [editForm, setEditForm] = React.useState({
    username: "",
    email: "",
  })
  const [isSaving, setIsSaving] = React.useState(false)

  React.useEffect(() => {
    getUrls()
      .then((data) => setUrlCount(data.length))
      .catch(() => setUrlCount(0))
  }, [])

  const startEdit = () => {
    if (!user) return
    setEditForm({ username: user.username, email: user.email })
    setEditing(true)
  }

  const cancelEdit = () => setEditing(false)

  const saveEdit = async () => {
    if (!user) return
    setIsSaving(true)
    try {
      const updated = await updateUser(user.id, editForm)
      localStorage.setItem(USER_ID_KEY, updated.id)
      setUser(updated)
      setEditing(false)
      toast.success("Profile updated.")
    } catch {
      toast.error("Failed to save changes.")
    } finally {
      setIsSaving(false)
    }
  }

  if (!user) return null

  return (
    <AppLayout title="Profile">
      <div className="mx-auto max-w-xl page-enter">
        <div className="rounded-xl border bg-card p-8 shadow-sm">
          {/* Avatar */}
          <div className="mb-6 flex flex-col items-center gap-3">
            <div className="flex size-20 items-center justify-center rounded-full bg-notion-blue text-2xl font-bold text-white">
              {getInitials(user.username)}
            </div>

            {editing ? (
              <div className="w-full space-y-3">
                <div className="space-y-1.5">
                  <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                    Username
                  </label>
                  <Input
                    value={editForm.username}
                    onChange={(e) =>
                      setEditForm((f) => ({ ...f, username: e.target.value }))
                    }
                    autoFocus
                  />
                </div>
                <div className="space-y-1.5">
                  <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                    Email
                  </label>
                  <Input
                    type="email"
                    value={editForm.email}
                    onChange={(e) =>
                      setEditForm((f) => ({ ...f, email: e.target.value }))
                    }
                  />
                </div>
                <div className="flex gap-2 pt-1">
                  <Button
                    size="sm"
                    onClick={saveEdit}
                    disabled={isSaving}
                  >
                    {isSaving ? <Spinner className="size-3.5" /> : <Check className="size-3.5" />}
                    Save
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={cancelEdit}
                    disabled={isSaving}
                  >
                    <X className="size-3.5" />
                    Cancel
                  </Button>
                </div>
              </div>
            ) : (
              <div className="text-center">
                <h2 className="text-xl font-semibold">{user.username}</h2>
                <p className="mt-0.5 text-sm text-muted-foreground">
                  {user.email}
                </p>
                <Button
                  variant="ghost"
                  size="sm"
                  className="mt-3"
                  onClick={startEdit}
                >
                  <Pencil className="size-3.5" />
                  Edit profile
                </Button>
              </div>
            )}
          </div>

          {/* Stats */}
          <div className="mt-2 grid grid-cols-2 gap-3 border-t pt-6">
            <div className="rounded-lg bg-background px-4 py-3 text-center border">
              <p className="text-2xl font-bold">
                {urlCount === null ? (
                  <Spinner className="mx-auto size-5 text-muted-foreground" />
                ) : (
                  urlCount
                )}
              </p>
              <p className="mt-0.5 text-xs text-muted-foreground">
                Links shortened
              </p>
            </div>
            <div className="rounded-lg bg-background px-4 py-3 text-center border">
              <p className="text-2xl font-bold">
                {format(new Date(user.created_at), "MMM yyyy")}
              </p>
              <p className="mt-0.5 text-xs text-muted-foreground">
                Member since
              </p>
            </div>
          </div>
        </div>
      </div>
    </AppLayout>
  )
}
