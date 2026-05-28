'use client'

import { createClient, SupabaseClient } from '@supabase/supabase-js'

let client: SupabaseClient | null = null

export function getSupabaseClient(): SupabaseClient {
  if (!client) {
    client = createClient(
      process.env.NEXT_PUBLIC_SUPABASE_URL!,
      process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY!
    )
  }
  return client
}

export async function login(email: string, password: string) {
  const supabase = getSupabaseClient()
  const { data, error } = await supabase.auth.signInWithPassword({ email, password })
  return { data, error }
}

export async function logout() {
  const supabase = getSupabaseClient()
  await supabase.auth.signOut()
}

export async function getClientSession() {
  const supabase = getSupabaseClient()
  const { data: { session } } = await supabase.auth.getSession()
  if (!session) return null

  // Fetch teacher profile
  const res = await fetch('/api/auth/me', {
    headers: { Authorization: `Bearer ${session.access_token}` }
  })
  if (!res.ok) return null
  return res.json()
}
