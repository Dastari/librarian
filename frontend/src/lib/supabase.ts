import { createClient, type SupabaseClient } from '@supabase/supabase-js'

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY

// Validate environment variables
if (!supabaseUrl || !supabaseAnonKey) {
  console.error(
    '⚠️ Missing Supabase environment variables!\n' +
    'Create a .env file in the frontend directory with:\n' +
    '  VITE_SUPABASE_URL=http://127.0.0.1:54321\n' +
    '  VITE_SUPABASE_ANON_KEY=your-anon-key\n\n' +
    'Get these values from: supabase status'
  )
}

// Create client only if we have the required values
export const supabase: SupabaseClient = supabaseUrl && supabaseAnonKey
  ? createClient(supabaseUrl, supabaseAnonKey)
  : (null as unknown as SupabaseClient) // Will cause errors if used without config

export const isSupabaseConfigured = Boolean(supabaseUrl && supabaseAnonKey)

export type { User, Session } from '@supabase/supabase-js'
