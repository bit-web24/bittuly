import { Link, useLocation } from "react-router-dom"
import { Link2 } from "lucide-react"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"
import { useAuth } from "@/context/AuthContext"

function getInitials(name: string): string {
  return name
    .split(" ")
    .map((n) => n[0])
    .join("")
    .toUpperCase()
    .slice(0, 2)
}

export function AppSidebar() {
  const location = useLocation()
  const { user } = useAuth()

  const navItems = [
    { label: "My Links", href: "/dashboard", icon: Link2 },
  ]

  return (
    <Sidebar collapsible="offcanvas">
      <SidebarHeader className="px-4 py-5">
        <span className="text-lg font-bold tracking-tight text-foreground">
          Bittuly
        </span>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.map((item) => (
                <SidebarMenuItem key={item.href}>
                  <SidebarMenuButton
                    asChild
                    isActive={location.pathname === item.href}
                    tooltip={item.label}
                  >
                    <Link to={item.href}>
                      <item.icon />
                      <span>{item.label}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      {user && (
        <SidebarFooter className="px-3 py-3">
          <div className="flex items-center gap-3 rounded-md px-2 py-2">
            <div className="flex size-8 shrink-0 items-center justify-center rounded-full bg-notion-blue text-xs font-semibold text-white">
              {getInitials(user.username)}
            </div>
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium">{user.username}</p>
              <p className="truncate text-xs text-muted-foreground">
                {user.email}
              </p>
            </div>
          </div>
        </SidebarFooter>
      )}
    </Sidebar>
  )
}
