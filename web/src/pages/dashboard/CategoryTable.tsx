// Progress per category, one row per roadmap category, sortable by any column.
// Unsorted it keeps the roadmap's order — the order the problems are meant to
// be worked through.

import { useMemo, useState } from 'react'
import type { Stats } from '@/api/types'
import styles from './dashboard.module.css'

type Row = Stats['by_category'][number] & { order: number; left: number; pct: number }
type Key = 'order' | 'category' | 'solved' | 'attempted' | 'left' | 'pct'

const COLUMNS: { key: Exclude<Key, 'order'>; label: string; numeric: boolean }[] = [
  { key: 'category', label: 'Category', numeric: false },
  { key: 'solved', label: 'Solved', numeric: true },
  { key: 'attempted', label: 'Attempted', numeric: true },
  { key: 'left', label: 'Left', numeric: true },
  { key: 'pct', label: 'Progress', numeric: true },
]

export function CategoryTable({ rows }: { rows: Stats['by_category'] }) {
  const [sort, setSort] = useState<{ key: Key; asc: boolean }>({ key: 'order', asc: true })

  const sorted = useMemo(() => {
    const all: Row[] = rows.map((r, order) => ({
      ...r,
      order,
      left: r.total - r.solved,
      pct: r.total ? r.solved / r.total : 0,
    }))
    const dir = sort.asc ? 1 : -1
    return all.sort((a, b) => {
      const k = sort.key
      const cmp = k === 'category' ? a.category.localeCompare(b.category) : a[k] - b[k]
      // Ties fall back to roadmap order, so equal rows never shuffle.
      return cmp * dir || a.order - b.order
    })
  }, [rows, sort])

  const toggle = (key: Exclude<Key, 'order'>, numeric: boolean) =>
    setSort((s) =>
      s.key === key
        ? { key, asc: !s.asc }
        : // Numbers start biggest-first: "where have I done the most" is the usual question.
          { key, asc: !numeric },
    )

  return (
    <>
      <div className={styles.tableScroll}>
        <table className={styles.table}>
          <thead>
            <tr>
              {COLUMNS.map((c) => (
                <th
                  key={c.key}
                  scope="col"
                  className={c.numeric ? styles.num : undefined}
                  aria-sort={sort.key === c.key ? (sort.asc ? 'ascending' : 'descending') : undefined}
                >
                  <button type="button" className={styles.sortBtn} onClick={() => toggle(c.key, c.numeric)}>
                    {c.label}
                    <span className={styles.sortMark} aria-hidden>
                      {sort.key === c.key ? (sort.asc ? '▲' : '▼') : '↕'}
                    </span>
                  </button>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {sorted.map((r) => (
              <tr key={r.category}>
                <th scope="row" className={styles.rowHead}>
                  {r.category}
                </th>
                <td className={styles.num}>{r.solved}</td>
                <td className={styles.num}>{r.attempted}</td>
                <td className={styles.num}>{r.left}</td>
                <td className={styles.progressCell}>
                  <span
                    className={styles.stack}
                    role="img"
                    aria-label={`${r.solved} of ${r.total} solved, ${r.attempted} attempted`}
                  >
                    <span className={styles.stackSolved} style={{ width: `${r.pct * 100}%` }} />
                    <span
                      className={styles.stackTried}
                      style={{ width: `${r.total ? (r.attempted / r.total) * 100 : 0}%` }}
                    />
                  </span>
                  <span className={styles.pct}>{Math.round(r.pct * 100)}%</span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {sort.key !== 'order' && (
        <button type="button" className={`mini-btn ${styles.resetSort}`} onClick={() => setSort({ key: 'order', asc: true })}>
          back to roadmap order
        </button>
      )}
    </>
  )
}
