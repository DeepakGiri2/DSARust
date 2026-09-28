// Visual QA for the visualizer (development builds only, routed at /dev/viz):
// any trace the Rust engine recorded into ../fixtures, driven by the real
// debugger, in either theme and at a chosen width. The fixture and width live
// in the query string so a reload — or a link — lands on the same picture.
//
// "hold t" freezes the transition into the current step at a chosen progress,
// so a mid-tween frame can be inspected (and compared with the desktop).
//
// Keys: ← back, → step in, space play/pause (when no control has focus).

import clsx from 'clsx'
import { useEffect, useMemo, useState } from 'react'
import { useSearchParams } from 'react-router'
import { useDebugger } from '@/debugger/useDebugger'
import type { Trace, VizView } from '@/trace/types'
import { Seg } from '@/ui'
import { VizCanvas } from '../VizCanvas'
import styles from './VizGalleryPage.module.css'

const loaders = import.meta.glob<Trace>('../fixtures/*.json', { import: 'default' })

interface Fixture {
  readonly name: string
  readonly load: () => Promise<Trace>
}

const FIXTURES: readonly Fixture[] = Object.entries(loaders)
  .map(([path, load]) => ({ name: path.slice(path.lastIndexOf('/') + 1, -'.json'.length), load }))
  .sort((a, b) => a.name.localeCompare(b.name))

/** `fill` tracks the stage; 470 is the desktop's canvas width in its screenshots. */
const WIDTHS = ['fill', '760', '470', '360'] as const
type WidthChoice = (typeof WIDTHS)[number]
type ThemeName = 'dark' | 'light'

const NO_VIEWS: VizView[] = []

/** The distinct view kinds a trace uses, stacks split by flavour, bar charts named as such. */
function kindsOf(trace: Trace | null): string[] {
  const kinds = new Set<string>()
  for (const step of trace?.steps ?? []) {
    for (const v of step.views) kinds.add(v.type === 'stack' ? v.kind : v.type === 'array' && v.bars ? 'bars' : v.type)
  }
  return [...kinds].sort()
}

/** The page theme, switchable here and restored when the gallery unmounts. */
function useThemeSwitch(): [ThemeName, (theme: ThemeName) => void] {
  const [theme, setTheme] = useState<ThemeName>(() =>
    document.documentElement.dataset.theme === 'light' ? 'light' : 'dark',
  )
  useEffect(() => {
    const root = document.documentElement
    const original = root.dataset.theme
    return () => {
      if (original === undefined) delete root.dataset.theme
      else root.dataset.theme = original
    }
  }, [])
  const apply = (next: ThemeName) => {
    document.documentElement.dataset.theme = next
    setTheme(next)
  }
  return [theme, apply]
}

export function Component() {
  const [params, setParams] = useSearchParams()
  const fixture = FIXTURES.find((f) => f.name === params.get('fixture')) ?? FIXTURES[0]
  const width: WidthChoice = WIDTHS.find((w) => w === params.get('width')) ?? 'fill'
  const setParam = (key: string, value: string) =>
    setParams(
      (current) => {
        const next = new URLSearchParams(current)
        next.set(key, value)
        return next
      },
      { replace: true },
    )

  const [loaded, setLoaded] = useState<{ name: string; trace: Trace } | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    let live = true
    setError(null)
    fixture.load().then(
      (trace) => {
        if (live) setLoaded({ name: fixture.name, trace })
      },
      (e: unknown) => {
        if (live) setError(`Could not load ${fixture.name}: ${String(e)}`)
      },
    )
    return () => {
      live = false
    }
  }, [fixture])
  const trace = loaded?.name === fixture.name ? loaded.trace : null
  const kinds = useMemo(() => kindsOf(trace), [trace])

  const [speed, setSpeed] = useState(1)
  const [heldT, setHeldT] = useState<number | null>(null)
  const [animate, setAnimate] = useState(true)
  const [theme, setTheme] = useThemeSwitch()
  const dbg = useDebugger(trace, { resetKey: fixture.name, speed, animate })
  const { stepIn, stepBack, togglePlay } = dbg

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.target instanceof Element && e.target.closest('input, select, textarea, button')) return
      if (e.key === 'ArrowRight') stepIn()
      else if (e.key === 'ArrowLeft') stepBack()
      else if (e.key === ' ') togglePlay()
      else return
      e.preventDefault()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [stepIn, stepBack, togglePlay])

  const step = dbg.step
  // Held: tween from the previous recorded step, whatever the debugger is doing.
  const from = heldT === null ? (dbg.fromStep !== step ? dbg.fromStep : null) : (trace?.steps[dbg.idx - 1] ?? null)
  const noTrace = trace === null

  return (
    <main className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>Visualizer gallery</h1>
        <label className={styles.field}>
          fixture
          <select
            className={clsx('input', styles.select)}
            value={fixture.name}
            onChange={(e) => setParam('fixture', e.target.value)}
          >
            {FIXTURES.map((f) => (
              <option key={f.name} value={f.name}>
                {f.name}
              </option>
            ))}
          </select>
        </label>
        <Seg
          small
          aria-label="Theme"
          value={theme}
          onChange={setTheme}
          options={[
            { value: 'dark', label: 'dark' },
            { value: 'light', label: 'light' },
          ]}
        />
        <Seg
          small
          aria-label="Canvas width"
          value={width}
          onChange={(w) => setParam('width', w)}
          options={WIDTHS.map((w) => ({ value: w, label: w === 'fill' ? 'fill' : `${w}px` }))}
        />
      </header>

      <div className={clsx('card', styles.transport)} role="toolbar" aria-label="Transport">
        <button type="button" className="mini-btn" aria-label="Restart" onClick={dbg.restart} disabled={noTrace || dbg.atStart}>
          ⏮
        </button>
        <button type="button" className="mini-btn" aria-label="Step back" onClick={dbg.stepBack} disabled={noTrace || dbg.atStart}>
          ◀
        </button>
        <button
          type="button"
          className="mini-btn"
          aria-label={dbg.playing ? 'Pause' : 'Play'}
          aria-pressed={dbg.playing}
          onClick={dbg.togglePlay}
          disabled={noTrace}
        >
          {dbg.playing ? '❚❚' : '▶'}
        </button>
        <button type="button" className="mini-btn" onClick={dbg.stepIn} disabled={noTrace || dbg.atEnd}>
          in
        </button>
        <button type="button" className="mini-btn" onClick={dbg.stepOver} disabled={noTrace || dbg.atEnd}>
          over
        </button>
        <button type="button" className="mini-btn" onClick={dbg.stepOut} disabled={noTrace || dbg.atEnd}>
          out
        </button>
        <input
          type="range"
          className={styles.scrub}
          aria-label="Step"
          min={0}
          max={Math.max(dbg.len - 1, 0)}
          value={dbg.idx}
          onChange={(e) => dbg.jumpTo(Number(e.target.value))}
          disabled={noTrace}
        />
        <span className={styles.counter}>
          {dbg.len === 0 ? 0 : dbg.idx + 1} / {dbg.len}
        </span>
        <label className={styles.field}>
          speed
          <input
            type="range"
            className={styles.speed}
            min={0.25}
            max={4}
            step={0.25}
            value={speed}
            onChange={(e) => setSpeed(Number(e.target.value))}
          />
          <span className="mono">{speed.toFixed(2)}x</span>
        </label>
        <label className="check">
          <input type="checkbox" checked={heldT !== null} onChange={(e) => setHeldT(e.target.checked ? 0.5 : null)} />
          hold t
        </label>
        {heldT !== null && (
          <label className={styles.field}>
            <input
              type="range"
              className={styles.speed}
              aria-label="Transition progress"
              min={0}
              max={1}
              step={0.01}
              value={heldT}
              onChange={(e) => setHeldT(Number(e.target.value))}
            />
            <span className="mono">t = {heldT.toFixed(2)}</span>
          </label>
        )}
        <label className="check">
          <input type="checkbox" checked={animate} onChange={(e) => setAnimate(e.target.checked)} />
          animate
        </label>
      </div>

      <p className={clsx('card', styles.note)}>{error ?? step?.note ?? 'Loading…'}</p>

      <section className={styles.stage} aria-label="Visualization">
        <div className={styles.frame} style={width === 'fill' ? undefined : { width: Number(width) }}>
          <VizCanvas
            views={step?.views ?? NO_VIEWS}
            prev={from?.views}
            t={heldT ?? dbg.t}
          />
        </div>
      </section>

      <footer className={styles.meta}>
        {step && (
          <>
            <span>
              tag <code>{step.tag}</code>
            </span>
            <span>depth {step.depth}</span>
            <span>{step.event}</span>
          </>
        )}
        {kinds.map((k) => (
          <span key={k} className="chip">
            {k}
          </span>
        ))}
        {dbg.atEnd && trace?.result !== undefined && (
          <span>
            result <code>{trace.result}</code>
          </span>
        )}
      </footer>
    </main>
  )
}
