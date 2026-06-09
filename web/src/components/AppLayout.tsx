import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar"
import { AppSidebar } from "./AppSidebar"
import { TopBar } from "./TopBar"

interface AppLayoutProps {
  title: string
  children: React.ReactNode
}

export function AppLayout({ title, children }: AppLayoutProps) {
  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset>
        <TopBar title={title} />
        <main className="flex-1 p-6 page-enter">{children}</main>
      </SidebarInset>
    </SidebarProvider>
  )
}
