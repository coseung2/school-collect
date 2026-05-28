import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createClient } from '@supabase/supabase-js'

const __dirname = dirname(fileURLToPath(import.meta.url))
const payload = JSON.parse(readFileSync(resolve(__dirname, 'register-payload.json'), 'utf8'))

function loadEnv(path) {
  const text = readFileSync(path, 'utf8')
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#') || !trimmed.includes('=')) continue
    const [key, ...rest] = trimmed.split('=')
    process.env[key] ||= rest.join('=').replace(/^['"]|['"]$/g, '')
  }
}

const projectRoot = resolve(__dirname, '../..')
loadEnv(resolve(projectRoot, '.env.local'))

const supabaseUrl = process.env.NEXT_PUBLIC_SUPABASE_URL
const serviceKey = process.env.SUPABASE_SERVICE_ROLE_KEY
if (!supabaseUrl || !serviceKey) throw new Error('Supabase env missing')

const sb = createClient(supabaseUrl, serviceKey, { auth: { persistSession: false } })

const { data: existing, error: findError } = await sb
  .from('form_templates')
  .select('id')
  .eq('title', payload.title)
  .eq('year', payload.year)
  .maybeSingle()
if (findError) throw findError

let templateId = existing?.id
if (!templateId) {
  const { data, error } = await sb
    .from('form_templates')
    .insert({ title: payload.title, description: payload.description, year: payload.year })
    .select('id')
    .single()
  if (error) throw error
  templateId = data.id
} else {
  const { error } = await sb
    .from('form_templates')
    .update({ description: payload.description, updated_at: new Date().toISOString() })
    .eq('id', templateId)
  if (error) throw error
  const { error: deleteError } = await sb.from('form_fields').delete().eq('template_id', templateId)
  if (deleteError) throw deleteError
}

const rows = payload.fields.map((field, index) => ({
  template_id: templateId,
  label: field.label,
  field_type: field.fieldType,
  required: field.required ?? false,
  options: field.options ?? null,
  order: field.order ?? index,
}))

const { error: insertError } = await sb.from('form_fields').insert(rows)
if (insertError) throw insertError

console.log(`template ready: ${templateId} (${rows.length} fields)`)
