import { apiRequest } from "./client"

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
  return apiRequest("/users/signup", {
    method: "POST",
    body: JSON.stringify(data),
  })
}

export async function verifyOtp(data: {
  pending_token: string
  otp: string
}): Promise<User> {
  return apiRequest("/users/verify-otp", {
    method: "POST",
    body: JSON.stringify(data),
  })
}

export async function login(data: {
  email: string
  password: string
}): Promise<User> {
  return apiRequest("/users/login", {
    method: "POST",
    body: JSON.stringify(data),
  })
}

export async function logout(): Promise<null> {
  return apiRequest("/users/logout", { method: "POST" })
}

export async function getUser(id: string): Promise<User> {
  return apiRequest(`/users/${id}`)
}

export async function updateUser(
  id: string,
  data: { username?: string; email?: string; password?: string }
): Promise<User> {
  return apiRequest(`/users/${id}`, {
    method: "PUT",
    body: JSON.stringify(data),
  })
}

export async function deleteUser(id: string): Promise<null> {
  return apiRequest(`/users/${id}`, { method: "DELETE" })
}
