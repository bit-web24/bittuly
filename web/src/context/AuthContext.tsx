import * as React from "react"
import { useNavigate } from "react-router-dom"
import { getUser, logout, type User } from "@/api/auth"

const USER_ID_KEY = "bittuly_user_id"

interface AuthContextValue {
  user: User | null
  isLoading: boolean
  setUser: (user: User | null) => void
  signOut: () => Promise<void>
}

const AuthContext = React.createContext<AuthContextValue | undefined>(undefined)

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = React.useState<User | null>(null)
  const [isLoading, setIsLoading] = React.useState(true)
  const navigate = useNavigate()

  React.useEffect(() => {
    const storedId = localStorage.getItem(USER_ID_KEY)
    if (!storedId) {
      setIsLoading(false)
      return
    }
    getUser(storedId)
      .then((u) => setUser(u))
      .catch(() => {
        localStorage.removeItem(USER_ID_KEY)
      })
      .finally(() => setIsLoading(false))
  }, [])

  const signOut = React.useCallback(async () => {
    try {
      await logout()
    } catch {
      // ignore — clear state regardless
    }
    localStorage.removeItem(USER_ID_KEY)
    setUser(null)
    navigate("/login")
  }, [navigate])

  const value = React.useMemo(
    () => ({ user, isLoading, setUser, signOut }),
    [user, isLoading, setUser, signOut]
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth(): AuthContextValue {
  const ctx = React.useContext(AuthContext)
  if (!ctx) throw new Error("useAuth must be used within AuthProvider")
  return ctx
}

export { USER_ID_KEY }
