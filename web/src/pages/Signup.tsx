import * as React from "react"
import { Link, useNavigate } from "react-router-dom"
import { Eye, EyeOff } from "lucide-react"
import { toast } from "sonner"
import { signup } from "@/api/auth"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"

export function Signup() {
  const navigate = useNavigate()
  const [form, setForm] = React.useState({
    username: "",
    email: "",
    password: "",
  })
  const [showPassword, setShowPassword] = React.useState(false)
  const [errors, setErrors] = React.useState<Record<string, string>>({})
  const [isLoading, setIsLoading] = React.useState(false)

  const validate = () => {
    const errs: Record<string, string> = {}
    if (form.username.length < 3) errs.username = "Minimum 3 characters."
    if (!form.email.includes("@")) errs.email = "Enter a valid email."
    if (form.password.length < 6) errs.password = "Minimum 6 characters."
    return errs
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    const errs = validate()
    if (Object.keys(errs).length) {
      setErrors(errs)
      return
    }
    setErrors({})
    setIsLoading(true)
    try {
      const { pending_token } = await signup(form)
      navigate("/verify-otp", {
        state: { pending_token, email: form.email },
      })
    } catch (err: unknown) {
      const e = err as { status?: number }
      if (e.status === 409) {
        setErrors({ email: "This email is already registered." })
      } else if (e.status === 422) {
        setErrors({ form: "Please check your inputs and try again." })
      } else {
        toast.error("Something went wrong. Please try again.")
      }
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="flex min-h-svh items-center justify-center bg-background px-4">
      <div className="w-full max-w-md page-enter">
        <div className="rounded-xl border bg-card p-8 shadow-sm">
          {/* Header */}
          <div className="mb-8 text-center">
            <h1 className="text-3xl font-bold tracking-tight">Bittuly</h1>
            <p className="mt-1.5 text-muted-foreground">Shorten smarter.</p>
          </div>

          <form onSubmit={handleSubmit} className="space-y-4">
            {/* Username */}
            <div className="space-y-1.5">
              <Label htmlFor="username">Username</Label>
              <Input
                id="username"
                placeholder="johndoe"
                value={form.username}
                onChange={(e) =>
                  setForm((f) => ({ ...f, username: e.target.value }))
                }
                aria-invalid={!!errors.username}
                autoComplete="username"
                autoFocus
              />
              {errors.username && (
                <p className="text-xs text-destructive">{errors.username}</p>
              )}
            </div>

            {/* Email */}
            <div className="space-y-1.5">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                placeholder="you@example.com"
                value={form.email}
                onChange={(e) =>
                  setForm((f) => ({ ...f, email: e.target.value }))
                }
                aria-invalid={!!errors.email}
                autoComplete="email"
              />
              {errors.email && (
                <p className="text-xs text-destructive">{errors.email}</p>
              )}
            </div>

            {/* Password */}
            <div className="space-y-1.5">
              <Label htmlFor="password">Password</Label>
              <div className="relative">
                <Input
                  id="password"
                  type={showPassword ? "text" : "password"}
                  placeholder="At least 6 characters"
                  value={form.password}
                  onChange={(e) =>
                    setForm((f) => ({ ...f, password: e.target.value }))
                  }
                  aria-invalid={!!errors.password}
                  autoComplete="new-password"
                  className="pr-10"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword((s) => !s)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                  aria-label={showPassword ? "Hide password" : "Show password"}
                >
                  {showPassword ? (
                    <EyeOff className="size-4" />
                  ) : (
                    <Eye className="size-4" />
                  )}
                </button>
              </div>
              {errors.password && (
                <p className="text-xs text-destructive">{errors.password}</p>
              )}
            </div>

            {errors.form && (
              <p className="text-sm text-destructive">{errors.form}</p>
            )}

            <Button
              type="submit"
              className="mt-2 w-full"
              disabled={isLoading}
            >
              {isLoading ? "Creating account…" : "Continue with email"}
            </Button>
          </form>

          {/* Divider */}
          <div className="my-6 flex items-center gap-3">
            <div className="h-px flex-1 bg-border" />
            <span className="text-xs text-muted-foreground">or</span>
            <div className="h-px flex-1 bg-border" />
          </div>

          <p className="text-center text-sm text-muted-foreground">
            Already have an account?{" "}
            <Link
              to="/login"
              className="font-medium text-notion-blue hover:underline"
            >
              Sign in →
            </Link>
          </p>
        </div>
      </div>
    </div>
  )
}
