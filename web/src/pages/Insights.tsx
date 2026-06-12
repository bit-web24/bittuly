import * as React from "react"
import { getUrls, type ShortenedUrl } from "@/api/urls"
import { AppLayout } from "@/components/AppLayout"
import {
  PieChart,
  Pie,
  Cell,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts"

const COLORS = [
  "#2563eb", // blue-600
  "#16a34a", // green-600
  "#ea580c", // orange-600
  "#dc2626", // red-600
  "#9333ea", // purple-600
  "#0891b2", // cyan-600
  "#d97706", // amber-600
  "#be185d", // pink-700
]

export function Insights() {
  const [urls, setUrls] = React.useState<ShortenedUrl[]>([])
  const [isLoading, setIsLoading] = React.useState(true)

  React.useEffect(() => {
    let mounted = true
    getUrls()
      .then((data) => {
        if (mounted) {
          setUrls(data)
          setIsLoading(false)
        }
      })
      .catch(() => {
        if (mounted) setIsLoading(false)
      })
    return () => {
      mounted = false
    }
  }, [])

  // Prepare data for the pie chart
  // Filter out URLs with 0 clicks to keep the chart clean, 
  // or you can leave them if you want to show everything.
  // Here we filter > 0 and sort by top clicks.
  const chartData = urls
    .filter((url) => url.click_count > 0)
    .sort((a, b) => b.click_count - a.click_count)
    .map((url) => ({
      name: url.short_code,
      value: url.click_count,
      original_url: url.original_url,
    }))

  const totalClicks = urls.reduce((sum, url) => sum + url.click_count, 0)

  return (
    <AppLayout title="Insights">
      <div className="mx-auto max-w-4xl space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold tracking-tight">Click Analytics</h1>
        </div>

        {isLoading ? (
          <div className="flex h-64 items-center justify-center text-muted-foreground">
            Loading analytics...
          </div>
        ) : urls.length === 0 ? (
          <div className="flex h-64 flex-col items-center justify-center rounded-lg border border-dashed bg-card text-center">
            <h3 className="text-lg font-medium text-foreground">No links yet</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Create a shortened link in the dashboard to start tracking clicks.
            </p>
          </div>
        ) : chartData.length === 0 ? (
          <div className="flex h-64 flex-col items-center justify-center rounded-lg border border-dashed bg-card text-center">
            <h3 className="text-lg font-medium text-foreground">No clicks yet</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Share your links to start seeing analytics. Total links: {urls.length}
            </p>
          </div>
        ) : (
          <div className="grid gap-6 md:grid-cols-2">
            <div className="rounded-lg border bg-card p-6 shadow-sm">
              <h2 className="mb-4 text-lg font-medium">Clicks by Link</h2>
              <div className="h-[300px] w-full">
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Pie
                      data={chartData}
                      cx="50%"
                      cy="50%"
                      innerRadius={60}
                      outerRadius={80}
                      paddingAngle={2}
                      dataKey="value"
                    >
                      {chartData.map((_, index) => (
                        <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                      ))}
                    </Pie>
                    <Tooltip 
                      formatter={(value: any) => [`${value} clicks`, 'Count']}
                      labelFormatter={(label) => `Link: ${label}`}
                    />
                    <Legend />
                  </PieChart>
                </ResponsiveContainer>
              </div>
            </div>

            <div className="rounded-lg border bg-card p-6 shadow-sm">
              <h2 className="mb-4 text-lg font-medium">Summary</h2>
              <dl className="space-y-4">
                <div className="flex items-center justify-between border-b pb-4">
                  <dt className="text-sm text-muted-foreground">Total Links</dt>
                  <dd className="text-2xl font-semibold">{urls.length}</dd>
                </div>
                <div className="flex items-center justify-between border-b pb-4">
                  <dt className="text-sm text-muted-foreground">Total Clicks</dt>
                  <dd className="text-2xl font-semibold">{totalClicks}</dd>
                </div>
                <div className="flex items-center justify-between pb-2">
                  <dt className="text-sm text-muted-foreground">Top Performing Link</dt>
                  <dd className="text-right">
                    <div className="text-lg font-semibold">{chartData[0].name}</div>
                    <div className="text-xs text-muted-foreground">{chartData[0].value} clicks</div>
                  </dd>
                </div>
              </dl>
            </div>
          </div>
        )}
      </div>
    </AppLayout>
  )
}
