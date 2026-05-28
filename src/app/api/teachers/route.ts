import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'
import { getAuthSession } from '@/lib/auth-api'
import { createClient } from '@supabase/supabase-js'

const supabaseUrl = process.env.NEXT_PUBLIC_SUPABASE_URL!
const serviceRoleKey = process.env.SUPABASE_SERVICE_ROLE_KEY!

/** 관리자 전용 - 요청자 확인 */
async function requireAdmin(req: NextRequest) {
  const auth = req.headers.get('Authorization')
  if (!auth?.startsWith('Bearer ')) return null
  const session = await getAuthSession(auth.slice(7))
  if (!session || session.teacher.role !== 'admin') return null
  return session
}

export async function GET(req: NextRequest) {
  const session = await requireAdmin(req)
  if (!session) {
    return NextResponse.json({ error: '관리자 권한 필요' }, { status: 403 })
  }

  const sb = getServiceSupabase()

  // Get all teachers with school info
  const { data: teachers } = await sb
    .from('sc_teachers')
    .select('*, school:sc_schools(name)')
    .eq('school_id', session.teacher.schoolId)
    .order('name')

  if (!teachers) return NextResponse.json([])

  // Get department assignments for all teachers
  const teacherIds = teachers.map(t => t.id)
  const { data: depts } = await sb
    .from('sc_teacher_depts')
    .select('*')
    .in('teacher_id', teacherIds)

  const result = teachers.map(t => ({
    ...t,
    departments: depts?.filter(d => d.teacher_id === t.id).map(d => d.department_name) || [],
  }))

  return NextResponse.json(result)
}

export async function POST(req: NextRequest) {
  const admin = await requireAdmin(req)
  if (!admin) {
    return NextResponse.json({ error: '관리자 권한 필요' }, { status: 403 })
  }

  const sb = getServiceSupabase()
  const body = await req.json()
  const { name, email, password, role, departments } = body

  if (!name || !email || !password) {
    return NextResponse.json({ error: '이름, 이메일, 비밀번호는 필수입니다.' }, { status: 400 })
  }

  // 1. Create auth user
  const authSb = createClient(supabaseUrl, serviceRoleKey, {
    auth: { autoRefreshToken: false, persistSession: false }
  })

  const { data: authUser, error: authError } = await authSb.auth.admin.createUser({
    email,
    password,
    email_confirm: true,
    user_metadata: { name, school_id: admin.teacher.schoolId },
  })

  if (authError) {
    return NextResponse.json({ error: authError.message }, { status: 400 })
  }

  // 2. Create teacher profile
  const { data: teacher, error: teacherError } = await sb
    .from('sc_teachers')
    .insert({
      auth_user_id: authUser.user.id,
      school_id: admin.teacher.schoolId,
      name,
      email,
      role: role || 'teacher',
    })
    .select()
    .single()

  if (teacherError) {
    return NextResponse.json({ error: teacherError.message }, { status: 500 })
  }

  // 3. Assign departments
  if (departments && departments.length > 0) {
    const deptInserts = departments.map((dept: string, i: number) => ({
      teacher_id: teacher.id,
      department_name: dept,
      is_primary: i === 0,
    }))
    await sb.from('sc_teacher_depts').insert(deptInserts)
  }

  return NextResponse.json({ ...teacher, departments: departments || [] }, { status: 201 })
}

export async function PATCH(req: NextRequest) {
  const admin = await requireAdmin(req)
  if (!admin) {
    return NextResponse.json({ error: '관리자 권한 필요' }, { status: 403 })
  }

  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, name, role, departments, isActive } = body

  const updates: Record<string, unknown> = {}
  if (name) updates.name = name
  if (role) updates.role = role
  if (typeof isActive === 'boolean') updates.is_active = isActive

  if (Object.keys(updates).length > 0) {
    const { error } = await sb.from('sc_teachers').update(updates).eq('id', id)
    if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  }

  if (departments !== undefined) {
    await sb.from('sc_teacher_depts').delete().eq('teacher_id', id)
    if (departments.length > 0) {
      const deptInserts = departments.map((dept: string, i: number) => ({
        teacher_id: id,
        department_name: dept,
        is_primary: i === 0,
      }))
      await sb.from('sc_teacher_depts').insert(deptInserts)
    }
  }

  return NextResponse.json({ success: true })
}

export async function DELETE(req: NextRequest) {
  const admin = await requireAdmin(req)
  if (!admin) {
    return NextResponse.json({ error: '관리자 권한 필요' }, { status: 403 })
  }

  const sb = getServiceSupabase()
  const body = await req.json()
  const { id } = body

  await sb.from('sc_teachers').update({ is_active: false }).eq('id', id)

  return NextResponse.json({ success: true })
}
