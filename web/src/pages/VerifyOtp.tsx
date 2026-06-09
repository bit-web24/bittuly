import * as React from "react"
import { Link, useNavigate, useLocation } from "react-router-dom"
import { verifyOtp } from "@/api/auth"
import { useAuth, USER_ID_KEY } from "@/context/AuthContext"
import { Button } from "@/components/ui/button"
import {
  InputOTP,
  InputOTPGroup,
  InputOTPSlot,
} from "@/components/ui/input-otp"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"

interface LocationState {
  pending_token?: string
  email?: string
}

export function VerifyOtp() {
  const navigate = useNavigate()
  const location = useLocation()
  const { setUser } = useAuth()
  const state = (location.state ?? {}) as LocationState

  // Guard: redirect if no pending token
  React.useEffect(() => {
    if (!state.pending_token) {
      navigate("/signup", { replace: true })
    }
  }, [state.pending_token, navigate])

  const [otp, setOtp] = React.useState("")
  const [error, setError] = React.useState("")
  const [isLoading, setIsLoading] = React.useState(false)
  const [shaking, setShaking] = React.useState(false)

  const triggerShake = () => {
    setShaking(true)
    setTimeout(() => setShaking(false), 500)
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (otp.length !== 6) {
      setError("Code must be exactly 6 digits.")
      triggerShake()
      return
    }
    setError("")
    setIsLoading(true)
    try {
      const user = await verifyOtp({
        pending_token: state.pending_token!,
        otp,
      })
      localStorage.setItem(USER_ID_KEY, user.id)
      setUser(user)
      navigate("/dashboard")
    } catch (err: unknown) {
      const e = err as { status?: number }
      if (e.status === 422) {
        setError("Code must be exactly 6 digits.")
        triggerShake()
      } else {
        setError("Incorrect code. Please try again.")
        triggerShake()
        setOtp("")
      }
    } finally {
      setIsLoading(false)
    }
  }

  if (!state.pending_token) return null

  return (
    <div className="flex min-h-svh items-center justify-center bg-background px-4">
      <div className="w-full max-w-md page-enter">
        <div className="rounded-xl border bg-card p-8 shadow-sm">
          {/* Header */}
          <div className="mb-8 text-center">
            <h1 className="text-3xl font-bold tracking-tight">Bittuly</h1>
            <p className="mt-3 text-xl font-semibold">Check your email</p>
            <p className="mt-2 text-sm text-muted-foreground">
              We sent a 6-digit code to{" "}
              <strong className="text-foreground">{state.email}</strong>. It
              expires in 10 minutes.
            </p>
          </div>

          <form onSubmit={handleSubmit} className="space-y-6">
            {/* OTP input */}
            <div className="flex justify-center">
              <div className={shaking ? "animate-shake" : ""}>
                <InputOTP
                  maxLength={6}
                  value={otp}
                  onChange={(val) => {
                    setOtp(val)
                    if (error) setError("")
                  }}
                >
                  <InputOTPGroup>
                    <InputOTPSlot index={0} />
                    <InputOTPSlot index={1} />
                    <InputOTPSlot index={2} />
                    <InputOTPSlot index={3} />
                    <InputOTPSlot index={4} />
                    <InputOTPSlot index={5} />
                  </InputOTPGroup>
                </InputOTP>
              </div>
            </div>

            {error && (
              <p className="text-center text-sm text-destructive">{error}</p>
            )}

            <Button
              type="submit"
              className="w-full"
              disabled={otp.length < 6 || isLoading}
            >
              {isLoading ? "Verifying…" : "Verify"}
            </Button>
          </form>

          {/* Footer links */}
          <div className="mt-6 flex items-center justify-between text-sm text-muted-foreground">
            <Link
              to="/signup"
              className="hover:text-foreground hover:underline"
            >
              Wrong email? Go back
            </Link>

            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <span className="cursor-not-allowed opacity-50">
                    Didn't receive it? Resend
                  </span>
                </TooltipTrigger>
                <TooltipContent>Resend not yet available</TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>
        </div>
      </div>
    </div>
  )
}
