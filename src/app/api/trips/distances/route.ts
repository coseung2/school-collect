import { NextRequest, NextResponse } from 'next/server'
import { getServiceSupabase } from '@/lib/supabase'

export async function GET() {
  const sb = getServiceSupabase()
  const { data, error } = await sb.from('trip_distance_std')
    .select('*')
    .order('region')

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json(data || [])
}

export async function POST(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { region, baseDistance, roundTrip } = body

  if (!region || !baseDistance) {
    return NextResponse.json({ error: 'region and baseDistance required' }, { status: 400 })
  }

  const { error } = await sb.from('trip_distance_std').insert({
    region,
    base_distance: baseDistance,
    round_trip: roundTrip !== false,
  })

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ ok: true }, { status: 201 })
}

export async function PATCH(req: NextRequest) {
  const sb = getServiceSupabase()
  const body = await req.json()
  const { id, ...updates } = body

  if (!id) return NextResponse.json({ error: 'id required' }, { status: 400 })

  const dbUpdates: any = {}
  if (updates.region) dbUpdates.region = updates.region
  if (updates.baseDistance) dbUpdates.base_distance = updates.baseDistance
  if (updates.roundTrip !== undefined) dbUpdates.round_trip = updates.roundTrip

  const { error } = await sb.from('trip_distance_std')
    .update(dbUpdates)
    .eq('id', id)

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ ok: true })
}

export async function DELETE(req: NextRequest) {
  const { searchParams } = new URL(req.url)
  const id = searchParams.get('id')
  if (!id) return NextResponse.json({ error: 'id required' }, { status: 400 })

  const sb = getServiceSupabase()
  const { error } = await sb.from('trip_distance_std')
    .delete()
    .eq('id', id)

  if (error) return NextResponse.json({ error: error.message }, { status: 500 })
  return NextResponse.json({ ok: true })
}
