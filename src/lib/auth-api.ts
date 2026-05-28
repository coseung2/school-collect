import { createClient } from '@supabase/supabase-js'
import type { AuthSession } from '@/lib/types'

const supabaseUrl = process.env.NEXT_PUBLIC_SUPABASE_URL!
const serviceRoleKey = process.env.SUPABASE_SERVICE_ROLE_KEY!

/**
 * API 라우트에서 Authorization 헤더의 Bearer token을 검증하고
 * 교사 프로필을 반환합니다.
 */
export async function getAuthSession(token: string): Promise<AuthSession | null> {
  try {
    const sb = createClient(supabaseUrl, serviceRoleKey, {
      auth: { autoRefreshToken: false, persistSession: false }
    })

    const { data: { user }, error } = await sb.auth.getUser(token)
    if (error || !user) return null

    const { data: teacher } = await sb
      .from('sc_teachers')
      .select('*, school:sc_schools(*)')
      .eq('auth_user_id', user.id)
      .eq('is_active', true)
      .single()

    if (!teacher) return null

    // Get department assignments
    const { data: depts } = await sb
      .from('sc_teacher_depts')
      .select('department_name')
      .eq('teacher_id', teacher.id)

    return {
      user: { id: user.id, email: user.email || '' },
      teacher: {
        ...teacher,
        departments: depts?.map(d => d.department_name) || [],
      }
    }
  } catch {
    return null
  }
}
