import { apiRequest, AUTH_BASE_URL } from "./client"

export interface User {
  id: string
  username: string
  email: string
  created_at: string
  updated_at: string
}

export interface SignupResponse {
  pending_token: string
}

export async function signup(data: {
  username: string
  email: string
  password: string
}): Promise<SignupResponse> {
  return apiRequest(AUTH_BASE_URL, "/api/auth/signup", {
    method: "POST",
    body: JSON.stringify(data),
  })
}

export async function verifyOtp(data: {
  pending_token: string
  otp: string
}): Promise<User> {
  return apiRequest(AUTH_BASE_URL, "/api/auth/verify-otp", {
    method: "POST",
    body: JSON.stringify(data),
  })
}

export async function login(data: {
  email: string
  password: string
}): Promise<User> {
  return apiRequest(AUTH_BASE_URL, "/api/auth/login", {
    method: "POST",
    body: JSON.stringify(data),
  })
}

export async function logout(): Promise<null> {
  return apiRequest(AUTH_BASE_URL, "/api/auth/logout", { method: "POST" })
}

export async function getUser(id: string): Promise<User> {
  return apiRequest(AUTH_BASE_URL, `/api/auth/${id}`)
}

export async function updateUser(
  id: string,
  data: { username?: string; email?: string; password?: string }
): Promise<User> {
  return apiRequest(AUTH_BASE_URL, `/api/auth/${id}`, {
    method: "PUT",
    body: JSON.stringify(data),
  })
}

export async function deleteUser(id: string): Promise<null> {
  return apiRequest(AUTH_BASE_URL, `/api/auth/${id}`, { method: "DELETE" })
}
