import * as React from "react"
import { useNavigate } from "react-router-dom"
import { toast } from "sonner"
import { deleteUser } from "@/api/auth"
import { useAuth, USER_ID_KEY } from "@/context/AuthContext"
import { useTheme } from "@/components/theme-provider"
import { AppLayout } from "@/components/AppLayout"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
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
import { Separator } from "@/components/ui/separator"

function ThemeOption({
  value,
  current,
  onClick,
}: {
  value: "light" | "dark"
  current: string
  onClick: () => void
}) {
  const isActive = current === value
  return (
    <button
      type="button"
      onClick={onClick}
      className={`rounded-md border px-4 py-2 text-sm font-medium transition-all duration-100 ${
        isActive
          ? "border-primary bg-primary text-primary-foreground shadow-sm"
          : "border-border bg-background text-foreground hover:bg-accent"
      }`}
    >
      {value.charAt(0).toUpperCase() + value.slice(1)}
    </button>
  )
}

export function Settings() {
  const { user, setUser } = useAuth()
  const { theme, setTheme } = useTheme()
  const navigate = useNavigate()
  const [confirmUsername, setConfirmUsername] = React.useState("")
  const [isDeleting, setIsDeleting] = React.useState(false)

  const handleDeleteAccount = async () => {
    if (!user) return
    setIsDeleting(true)
    try {
      await deleteUser(user.id)
      localStorage.removeItem(USER_ID_KEY)
      setUser(null)
      navigate("/login")
    } catch {
      toast.error("Failed to delete account. Please try again.")
    } finally {
      setIsDeleting(false)
    }
  }

  if (!user) return null

  const canDelete = confirmUsername === user.username

  return (
    <AppLayout title="Settings">
      <div className="mx-auto max-w-xl space-y-6 page-enter">
        {/* Appearance */}
        <div className="rounded-xl border bg-card p-6 shadow-sm">
          <h2 className="mb-4 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
            Appearance
          </h2>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium">Theme</p>
              <p className="mt-0.5 text-xs text-muted-foreground">
                Choose your preferred colour scheme.
              </p>
            </div>
            <div className="flex gap-1.5">
              <ThemeOption
                value="light"
                current={theme}
                onClick={() => setTheme("light")}
              />
              <ThemeOption
                value="dark"
                current={theme}
                onClick={() => setTheme("dark")}
              />
            </div>
          </div>
        </div>

        {/* Account */}
        <div className="rounded-xl border bg-card p-6 shadow-sm">
          <h2 className="mb-4 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
            Account
          </h2>

          <Separator className="mb-5" />

          <div className="flex items-start justify-between gap-4">
            <div>
              <p className="text-sm font-medium text-destructive">
                Delete Account
              </p>
              <p className="mt-1 text-xs text-muted-foreground max-w-xs">
                Permanently delete your account and all associated data. This
                action cannot be undone.
              </p>
            </div>

            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="destructive" size="sm">
                  Delete Account
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Are you sure?</AlertDialogTitle>
                  <AlertDialogDescription asChild>
                    <div className="space-y-3">
                      <p>
                        Type your username to confirm:{" "}
                        <strong className="text-foreground">
                          {user.username}
                        </strong>
                      </p>
                      <Input
                        placeholder={user.username}
                        value={confirmUsername}
                        onChange={(e) => setConfirmUsername(e.target.value)}
                        autoFocus
                      />
                    </div>
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel onClick={() => setConfirmUsername("")}>
                    Cancel
                  </AlertDialogCancel>
                  <AlertDialogAction
                    variant="destructive"
                    onClick={handleDeleteAccount}
                    disabled={!canDelete || isDeleting}
                  >
                    {isDeleting ? "Deleting…" : "Delete my account"}
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          </div>
        </div>
      </div>
    </AppLayout>
  )
}
