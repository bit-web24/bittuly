import * as React from "react"
import { getUrls, type ShortenedUrl } from "@/api/urls"
import { AppLayout } from "@/components/AppLayout"
import {
  PieChart,
  Pie,
  Cell,
  Tooltip,
  ResponsiveContainer,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
} from "recharts"

const COLORS = [
  "#2563eb", "#16a34a", "#ea580c", "#dc2626",
  "#9333ea", "#0891b2", "#d97706", "#be185d",
]

const RADIAN = Math.PI / 180

// ---------- Custom renderers ----------

/** Renders the short code + % outside each pie slice. Skips tiny slices. */
function PieLabel({ cx, cy, midAngle, outerRadius, name, percent }: any) {
  if (percent < 0.05) return null
  const r = outerRadius + 26
  const x = cx + r * Math.cos(-midAngle * RADIAN)
  const y = cy + r * Math.sin(-midAngle * RADIAN)
  return (
    <text
      x={x}
      y={y}
      fontSize={11}
      fill="currentColor"
      textAnchor={x > cx ? "start" : "end"}
      dominantBaseline="central"
    >
      {`/${name} · ${(percent * 100).toFixed(0)}%`}
    </text>
  )
}

function PieTooltip({ active, payload }: any) {
  if (!active || !payload?.length) return null
  const d = payload[0].payload
  return (
    <div className="rounded-lg border bg-popover px-3 py-2.5 shadow-md text-sm space-y-1 max-w-[260px]">
      <p className="font-mono font-semibold text-foreground">/{d.name}</p>
      <p className="text-xs text-muted-foreground break-all line-clamp-2">{d.original_url}</p>
      <p className="font-medium text-foreground">
        {d.value} {d.value === 1 ? "click" : "clicks"}
        <span className="ml-1.5 text-muted-foreground text-xs">
          ({d.percent !== undefined ? (d.percent * 100).toFixed(1) : "?"}%)
        </span>
      </p>
    </div>
  )
}

function BarTooltip({ active, payload }: any) {
  if (!active || !payload?.length) return null
  const d = payload[0].payload
  return (
    <div className="rounded-lg border bg-popover px-3 py-2.5 shadow-md text-sm space-y-1 max-w-[260px]">
      <p className="font-mono font-semibold">/{d.name}</p>
      <p className="text-xs text-muted-foreground break-all line-clamp-2">{d.original_url}</p>
      <p className="font-medium">{d.clicks} {d.clicks === 1 ? "click" : "clicks"}</p>
    </div>
  )
}

// ---------- Stat card ----------

function StatCard({
  label,
  value,
  sub,
  highlight = false,
}: {
  label: string
  value: string | number
  sub?: string
  highlight?: boolean
}) {
  return (
    <div
      className={`rounded-xl border p-5 shadow-sm ${
        highlight
          ? "border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-950/40"
          : "bg-card"
      }`}
    >
      <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">{label}</p>
      <p className="mt-2 text-3xl font-bold tabular-nums text-foreground">{value}</p>
      {sub && <p className="mt-1 text-xs text-muted-foreground">{sub}</p>}
    </div>
  )
}

// ---------- Page ----------

export function Insights() {
  const [urls, setUrls] = React.useState<ShortenedUrl[]>([])
  const [isLoading, setIsLoading] = React.useState(true)

  React.useEffect(() => {
    let mounted = true
    getUrls()
      .then((data) => { if (mounted) { setUrls(data); setIsLoading(false) } })
      .catch(() => { if (mounted) setIsLoading(false) })
    return () => { mounted = false }
  }, [])

  // --- Derived stats ---
  const sorted = [...urls].sort((a, b) => b.click_count - a.click_count)
  const totalClicks = urls.reduce((s, u) => s + u.click_count, 0)
  const activeLinks = urls.filter((u) => u.click_count > 0)
  const silentLinks = urls.filter((u) => u.click_count === 0)
  const avgClicks = urls.length
    ? (totalClicks / urls.length).toFixed(1)
    : "—"
  const topLink = sorted[0]

  // Pie: top 7 by clicks; group the rest as "others"
  const MAX_PIE = 7
  const activeSorted = [...activeLinks].sort((a, b) => b.click_count - a.click_count)
  const pieTop = activeSorted.slice(0, MAX_PIE)
  const pieRest = activeSorted.slice(MAX_PIE)
  const otherClicks = pieRest.reduce((s, u) => s + u.click_count, 0)
  const pieData = [
    ...pieTop.map((u) => ({
      name: u.short_code,
      value: u.click_count,
      original_url: u.original_url,
    })),
    ...(otherClicks > 0
      ? [{ name: "others", value: otherClicks, original_url: `${pieRest.length} more link(s)` }]
      : []),
  ]

  // Bar: top 8 by clicks (includes 0-click links so you see "dead" links)
  const barData = sorted.slice(0, 8).map((u) => ({
    name: u.short_code,
    clicks: u.click_count,
    original_url: u.original_url,
  }))

  // ---------- Loading / empty states ----------

  if (isLoading) {
    return (
      <AppLayout title="Insights">
        <div className="flex h-64 items-center justify-center text-muted-foreground text-sm">
          Loading analytics…
        </div>
      </AppLayout>
    )
  }

  if (urls.length === 0) {
    return (
      <AppLayout title="Insights">
        <div className="flex h-64 flex-col items-center justify-center rounded-xl border border-dashed text-center px-6">
          <h3 className="text-lg font-semibold">No links yet</h3>
          <p className="mt-1 text-sm text-muted-foreground">
            Create your first shortened link to start seeing analytics here.
          </p>
        </div>
      </AppLayout>
    )
  }

  return (
    <AppLayout title="Insights">
      <div className="mx-auto max-w-5xl space-y-6">
        <h1 className="text-2xl font-semibold tracking-tight">Click Analytics</h1>



        {/* ── Stat cards ── */}
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
          <StatCard label="Total Links" value={urls.length} />
          <StatCard
            label="Total Clicks"
            value={totalClicks}
            highlight={totalClicks > 0}
          />
          <StatCard
            label="Active Links"
            value={activeLinks.length}
            sub={
              silentLinks.length > 0
                ? `${silentLinks.length} never clicked`
                : "All links have clicks"
            }
          />
          <StatCard
            label="Avg Clicks / Link"
            value={avgClicks}
            sub={
              topLink && topLink.click_count > 0
                ? `Best: /${topLink.short_code} (${topLink.click_count})`
                : "No clicks yet"
            }
          />
        </div>

        {activeLinks.length === 0 ? (
          <div className="flex h-48 flex-col items-center justify-center rounded-xl border border-dashed text-center px-6">
            <p className="text-sm text-muted-foreground">
              No clicks recorded yet. You have {urls.length}{" "}
              {urls.length === 1 ? "link" : "links"} — share them to see charts appear here.
            </p>
          </div>
        ) : (
          <>
            {/* ── Charts row ── */}
            <div className="grid gap-6 md:grid-cols-2">
              {/* Pie / donut */}
              <div className="rounded-xl border bg-card p-6 shadow-sm">
                <h2 className="text-base font-semibold">Clicks Distribution</h2>
                <p className="mt-0.5 mb-4 text-xs text-muted-foreground">
                  Each arc is a link — hover for full URL and count
                </p>
                <div className="h-[300px]">
                  <ResponsiveContainer width="100%" height="100%">
                    <PieChart margin={{ top: 24, right: 56, bottom: 24, left: 56 }}>
                      <Pie
                        data={pieData}
                        cx="50%"
                        cy="50%"
                        innerRadius={52}
                        outerRadius={72}
                        paddingAngle={2}
                        dataKey="value"
                        label={PieLabel}
                        labelLine={{ stroke: "#94a3b8", strokeWidth: 1 }}
                      >
                        {pieData.map((_, i) => (
                          <Cell key={i} fill={COLORS[i % COLORS.length]} />
                        ))}
                      </Pie>
                      <Tooltip content={<PieTooltip />} />
                    </PieChart>
                  </ResponsiveContainer>
                </div>
              </div>

              {/* Horizontal bar */}
              <div className="rounded-xl border bg-card p-6 shadow-sm">
                <h2 className="text-base font-semibold">Top Links by Clicks</h2>
                <p className="mt-0.5 mb-4 text-xs text-muted-foreground">
                  Up to 8 links ranked by total clicks
                </p>
                <div className="h-[300px]">
                  <ResponsiveContainer width="100%" height="100%">
                    <BarChart
                      data={barData}
                      layout="vertical"
                      margin={{ top: 4, right: 20, bottom: 4, left: 0 }}
                    >
                      <CartesianGrid strokeDasharray="3 3" horizontal={false} stroke="rgba(0,0,0,0.07)" />
                      <XAxis type="number" tick={{ fontSize: 11 }} allowDecimals={false} />
                      <YAxis
                        type="category"
                        dataKey="name"
                        tick={{ fontSize: 11 }}
                        tickFormatter={(v) => `/${v}`}
                        width={68}
                      />
                      <Tooltip content={<BarTooltip />} cursor={{ fill: "rgba(0,0,0,0.04)" }} />
                      <Bar dataKey="clicks" radius={[0, 4, 4, 0]} maxBarSize={22}>
                        {barData.map((_, i) => (
                          <Cell key={i} fill={COLORS[i % COLORS.length]} />
                        ))}
                      </Bar>
                    </BarChart>
                  </ResponsiveContainer>
                </div>
              </div>
            </div>

            {/* ── All links table ── */}
            <div className="rounded-xl border bg-card shadow-sm overflow-hidden">
              <div className="px-6 py-4 border-b">
                <h2 className="text-base font-semibold">All Links Performance</h2>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Sorted by most clicks · progress bar shows share of total clicks
                </p>
              </div>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead className="bg-muted/40">
                    <tr className="text-xs text-muted-foreground uppercase tracking-wide border-b">
                      <th className="px-5 py-3 text-left font-medium">Short Code</th>
                      <th className="px-5 py-3 text-left font-medium">Original URL</th>
                      <th className="px-5 py-3 text-right font-medium">Clicks</th>
                      <th className="px-5 py-3 text-right font-medium min-w-[140px]">Share</th>
                      <th className="px-5 py-3 text-right font-medium">Created</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border">
                    {sorted.map((url, i) => {
                      const share =
                        totalClicks > 0 ? (url.click_count / totalClicks) * 100 : 0
                      return (
                        <tr
                          key={url.id}
                          className="hover:bg-muted/30 transition-colors duration-100"
                        >
                          {/* Short code */}
                          <td className="px-5 py-3">
                            <div className="flex items-center gap-2">
                              <span
                                className="inline-block size-2 rounded-full shrink-0"
                                style={{ background: COLORS[i % COLORS.length] }}
                              />
                              <a
                                href={`http://localhost:3000/${url.short_code}`}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="font-mono font-medium text-notion-blue hover:underline"
                              >
                                /{url.short_code}
                              </a>
                            </div>
                          </td>

                          {/* Original URL */}
                          <td className="px-5 py-3 max-w-[200px]">
                            <span
                              title={url.original_url}
                              className="block truncate text-xs text-muted-foreground"
                            >
                              {url.original_url}
                            </span>
                          </td>

                          {/* Clicks */}
                          <td className="px-5 py-3 text-right tabular-nums font-semibold">
                            {url.click_count}
                          </td>

                          {/* Share with progress bar */}
                          <td className="px-5 py-3">
                            <div className="flex items-center justify-end gap-2">
                              <div className="h-1.5 w-20 rounded-full bg-muted overflow-hidden">
                                <div
                                  className="h-full rounded-full"
                                  style={{
                                    width: `${share}%`,
                                    background: COLORS[i % COLORS.length],
                                  }}
                                />
                              </div>
                              <span className="tabular-nums text-xs text-muted-foreground w-12 text-right">
                                {share > 0 ? `${share.toFixed(1)}%` : "—"}
                              </span>
                            </div>
                          </td>

                          {/* Created */}
                          <td className="px-5 py-3 text-right text-xs text-muted-foreground whitespace-nowrap">
                            {new Date(url.created_at).toLocaleDateString("en-IN", {
                              day: "numeric",
                              month: "short",
                              year: "numeric",
                            })}
                          </td>
                        </tr>
                      )
                    })}
                  </tbody>
                </table>
              </div>
            </div>
          </>
        )}
      </div>
    </AppLayout>
  )
}
