import * as React from "react"
import { RefreshCw, Server, Database, Zap, Clock, Tag } from "lucide-react"
import { getHealth, type HealthData } from "@/api/health"
import { AppLayout } from "@/components/AppLayout"
import { Button } from "@/components/ui/button"

// ── helpers ─────────────────────────────────────────────────────────────────

function formatUptime(secs: number): string {
  if (secs === 0) return "—"
  const d = Math.floor(secs / 86400)
  const h = Math.floor((secs % 86400) / 3600)
  const m = Math.floor((secs % 3600) / 60)
  const s = secs % 60
  if (d > 0) return `${d}d ${h}h ${m}m`
  if (h > 0) return `${h}h ${m}m ${s}s`
  if (m > 0) return `${m}m ${s}s`
  return `${s}s`
}

function formatTime(d: Date): string {
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })
}

// ── sub-components ───────────────────────────────────────────────────────────

interface StatusDotProps {
  ok: boolean
  pulse?: boolean
}
function StatusDot({ ok, pulse = false }: StatusDotProps) {
  return (
    <span className="relative flex size-3 shrink-0">
      {ok && pulse && (
        <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-400 opacity-60" />
      )}
      <span
        className={[
          "relative inline-flex size-3 rounded-full",
          ok ? "bg-green-500" : "bg-red-500",
        ].join(" ")}
      />
    </span>
  )
}

interface ServiceCardProps {
  icon: React.ElementType
  label: string
  value: string
}
function ServiceCard({ icon: Icon, label, value }: ServiceCardProps) {
  const ok = value === "ok"
  return (
    <div className="flex items-start gap-4 rounded-xl border bg-card p-5 shadow-sm transition-shadow hover:shadow-md">
      <div
        className={[
          "flex size-10 shrink-0 items-center justify-center rounded-lg",
          ok
            ? "bg-green-500/10 text-green-600 dark:text-green-400"
            : "bg-red-500/10 text-red-600 dark:text-red-400",
        ].join(" ")}
      >
        <Icon className="size-5" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <StatusDot ok={ok} pulse={ok} />
          <p className="text-sm font-semibold">{label}</p>
          <span
            className={[
              "ml-auto rounded-full px-2 py-0.5 text-xs font-medium",
              ok
                ? "bg-green-500/10 text-green-700 dark:text-green-400"
                : "bg-red-500/10 text-red-700 dark:text-red-400",
            ].join(" ")}
          >
            {ok ? "Online" : "Offline"}
          </span>
        </div>
        <p className="mt-1 truncate font-mono text-xs text-muted-foreground">{value}</p>
      </div>
    </div>
  )
}

interface StatCardProps {
  icon: React.ElementType
  label: string
  value: string
}
function StatCard({ icon: Icon, label, value }: StatCardProps) {
  return (
    <div className="flex items-center gap-3 rounded-xl border bg-card p-4 shadow-sm">
      <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted">
        <Icon className="size-4 text-muted-foreground" />
      </div>
      <div>
        <p className="text-xs text-muted-foreground">{label}</p>
        <p className="font-mono text-sm font-semibold">{value}</p>
      </div>
    </div>
  )
}

// ── main page ────────────────────────────────────────────────────────────────

const REFRESH_INTERVAL_MS = 30_000

export function Health() {
  const [health, setHealth] = React.useState<HealthData | null>(null)
  const [loading, setLoading] = React.useState(true)
  const [spinning, setSpinning] = React.useState(false)
  const [lastChecked, setLastChecked] = React.useState<Date | null>(null)
  const [countdown, setCountdown] = React.useState(REFRESH_INTERVAL_MS / 1000)

  const fetchHealth = React.useCallback(async (manual = false) => {
    if (manual) setSpinning(true)
    const { data } = await getHealth()
    setHealth(data)
    setLastChecked(new Date())
    setCountdown(REFRESH_INTERVAL_MS / 1000)
    setLoading(false)
    if (manual) setTimeout(() => setSpinning(false), 600)
  }, [])

  // Initial load + auto-refresh every 30 s
  React.useEffect(() => {
    fetchHealth()
    const interval = setInterval(() => fetchHealth(), REFRESH_INTERVAL_MS)
    return () => clearInterval(interval)
  }, [fetchHealth])

  // Countdown ticker
  React.useEffect(() => {
    const tick = setInterval(() => {
      setCountdown((c) => (c > 0 ? c - 1 : REFRESH_INTERVAL_MS / 1000))
    }, 1000)
    return () => clearInterval(tick)
  }, [])

  const isHealthy = health?.status === "healthy"

  return (
    <AppLayout title="System Health">
      <div className="mx-auto max-w-2xl space-y-6">

        {/* ── Overall status banner ─────────────────────────────────────── */}
        <div
          className={[
            "relative overflow-hidden rounded-2xl border p-6 shadow-sm",
            loading
              ? "border-border bg-card"
              : isHealthy
                ? "border-green-200 bg-green-50 dark:border-green-900 dark:bg-green-950/30"
                : "border-red-200 bg-red-50 dark:border-red-900 dark:bg-red-950/30",
          ].join(" ")}
        >
          {/* decorative glow */}
          {!loading && (
            <div
              className={[
                "pointer-events-none absolute -right-10 -top-10 size-40 rounded-full opacity-20 blur-3xl",
                isHealthy ? "bg-green-400" : "bg-red-400",
              ].join(" ")}
            />
          )}

          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              {loading ? (
                <span className="size-4 animate-spin rounded-full border-2 border-muted-foreground border-t-transparent" />
              ) : (
                <StatusDot ok={isHealthy} pulse={isHealthy} />
              )}
              <div>
                <p className="text-xs font-medium uppercase tracking-widest text-muted-foreground">
                  System Status
                </p>
                <p
                  className={[
                    "text-2xl font-bold capitalize",
                    loading
                      ? "text-muted-foreground"
                      : isHealthy
                        ? "text-green-700 dark:text-green-400"
                        : "text-red-700 dark:text-red-400",
                  ].join(" ")}
                >
                  {loading ? "Checking…" : health?.status}
                </p>
              </div>
            </div>

            <div className="flex flex-col items-end gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => fetchHealth(true)}
                disabled={spinning}
                className="gap-1.5"
              >
                <RefreshCw className={["size-3.5", spinning ? "animate-spin" : ""].join(" ")} />
                Refresh
              </Button>
              {lastChecked && (
                <p className="text-xs text-muted-foreground">
                  Next in {countdown}s
                </p>
              )}
            </div>
          </div>

          {lastChecked && (
            <p className="mt-3 text-xs text-muted-foreground">
              Last checked at {formatTime(lastChecked)}
            </p>
          )}
        </div>

        {/* ── Service cards ─────────────────────────────────────────────── */}
        {health && (
          <div className="space-y-3">
            <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
              Services
            </h2>
            <ServiceCard icon={Database} label="PostgreSQL" value={health.postgres} />
            <ServiceCard icon={Zap}      label="Redis"      value={health.redis} />
          </div>
        )}

        {/* ── Meta stats ────────────────────────────────────────────────── */}
        {health && (
          <div className="space-y-3">
            <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
              Server Info
            </h2>
            <div className="grid grid-cols-2 gap-3">
              <StatCard
                icon={Clock}
                label="Uptime"
                value={formatUptime(health.uptime_secs)}
              />
              <StatCard
                icon={Tag}
                label="Version"
                value={`v${health.version}`}
              />
              <StatCard
                icon={Server}
                label="API"
                value="localhost:3000"
              />
            </div>
          </div>
        )}

      </div>
    </AppLayout>
  )
}
