import { BrowserRouter } from "react-router-dom"
import { ThemeProvider } from "@/components/theme-provider"
import { AuthProvider } from "@/context/AuthContext"
import { AppRouter } from "@/router/AppRouter"
import { Toaster } from "@/components/ui/sonner"

export function App() {
  return (
    <BrowserRouter>
      <ThemeProvider storageKey="bittuly_theme" defaultTheme="light">
        <AuthProvider>
          <AppRouter />
          <Toaster position="bottom-right" richColors />
        </AuthProvider>
      </ThemeProvider>
    </BrowserRouter>
  )
}

export default App
