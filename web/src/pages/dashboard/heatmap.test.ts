import { describe, expect, it } from 'vitest'
import { buildHeatmap, describeCell, levelFor, todayIn } from './heatmap'

describe('activity heatmap', () => {
  it('knows what day it is in the account’s zone, not the browser’s', () => {
    const at = new Date('2026-09-26T02:00:00Z')
    expect(todayIn('UTC', at)).toBe('2026-09-26')
    expect(todayIn('America/Los_Angeles', at)).toBe('2026-09-25')
    expect(todayIn('Asia/Tokyo', at)).toBe('2026-09-26')
    // An unknown zone falls back instead of throwing.
    expect(todayIn('Not/AZone', at)).toMatch(/^\d{4}-\d{2}-\d{2}$/)
  })

  it('covers exactly the last year, ending today, one cell per day', () => {
    const map = buildHeatmap([], '2026-09-26')
    expect(map.days).toHaveLength(365)
    expect(map.days[364].day).toBe('2026-09-26')
    expect(map.days[0].day).toBe('2025-09-27')
    expect(new Set(map.days.map((d) => d.day)).size).toBe(365)
    expect(map.weeks.every((w) => w.length === 7)).toBe(true)
  })

  it('steps across daylight-saving changes without skipping or repeating a day', () => {
    // US clocks go back on 2026-11-01; Europe’s went back on 2026-10-25.
    const map = buildHeatmap([], '2026-11-03', 20)
    const days = map.days.map((d) => d.day)
    expect(days).toContain('2026-10-25')
    expect(days).toContain('2026-11-01')
    for (let i = 1; i < days.length; i++) {
      expect(Date.parse(days[i]) - Date.parse(days[i - 1])).toBe(86_400_000)
    }
  })

  it('pads the first week so every day sits under its weekday', () => {
    const map = buildHeatmap([], '2026-09-26') // a Saturday
    const firstWeek = map.weeks[0]
    const lead = firstWeek.findIndex((c) => c !== null)
    expect(new Date(`${map.days[0].day}T00:00:00Z`).getUTCDay()).toBe(lead)
    expect(map.weeks[map.weeks.length - 1][6]?.day).toBe('2026-09-26')
  })

  it('scales intensity to the busiest day and counts manual solves as activity', () => {
    expect(levelFor(0, 10)).toBe(0)
    expect(levelFor(1, 10)).toBe(1)
    expect(levelFor(10, 10)).toBe(4)
    const map = buildHeatmap(
      [
        { day: '2026-09-20', runs: 8, solved: 0 },
        { day: '2026-09-21', runs: 0, solved: 1 },
      ],
      '2026-09-26',
    )
    const cell = (day: string) => map.days.find((d) => d.day === day)
    expect(cell('2026-09-20')?.level).toBe(4)
    expect(cell('2026-09-21')?.level).toBe(1)
    expect(map.activeDays).toBe(2)
    expect(map.totalRuns).toBe(8)
    expect(map.totalSolved).toBe(1)
  })

  it('never crowds two month labels together', () => {
    const { months } = buildHeatmap([], '2026-09-26')
    for (let i = 1; i < months.length; i++) expect(months[i].column - months[i - 1].column).toBeGreaterThanOrEqual(3)
  })

  it('describes a day in words', () => {
    expect(describeCell({ day: '2026-09-22', runs: 3, solved: 1, level: 2 })).toMatch(/^3 runs · 1 solved — /)
    expect(describeCell({ day: '2026-09-22', runs: 1, solved: 0, level: 1 })).toMatch(/^1 run — /)
    expect(describeCell({ day: '2026-09-22', runs: 0, solved: 0, level: 0 })).toMatch(/^No activity — /)
  })
})
