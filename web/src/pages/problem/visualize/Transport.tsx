// The transport bar along the bottom (`Visualize::controls`).

import { memo } from 'react'
import clsx from 'clsx'
import ui from '../shared/ui.module.css'
import styles from './Visualize.module.css'

// The desktop's speed slider: 0.25×–6×, logarithmic, so the slow end where
// the differences matter gets most of the travel.
export const SPEED_MIN = 0.25
export const SPEED_MAX = 6
const SPEED_STEPS = 1000

export function speedToSlider(speed: number): number {
  const s = Math.min(SPEED_MAX, Math.max(SPEED_MIN, speed))
  return Math.round((SPEED_STEPS * Math.log(s / SPEED_MIN)) / Math.log(SPEED_MAX / SPEED_MIN))
}

export function sliderToSpeed(v: number): number {
  const raw = SPEED_MIN * (SPEED_MAX / SPEED_MIN) ** (v / SPEED_STEPS)
  return Math.round(raw * 100) / 100
}

export interface TransportProps {
  idx: number
  len: number
  playing: boolean
  speed: number
  animate: boolean
  consoleOpen: boolean
  onRestart: () => void
  onBack: () => void
  onPlay: () => void
  onIn: () => void
  onOver: () => void
  onOut: () => void
  onContinue: () => void
  onEnd: () => void
  onJump: (i: number) => void
  onSpeed: (speed: number) => void
  onAnimate: (on: boolean) => void
  onConsole: () => void
}

export const Transport = memo(function Transport(p: TransportProps) {
  const off = p.len === 0
  return (
    <div className={clsx(ui.toolbar, styles.transport)} role="toolbar" aria-label="Debugger controls">
      <button type="button" className={clsx('mini-btn', styles.tbtn)} disabled={off} onClick={p.onRestart} title="restart (R)" aria-label="Restart">
        ⏮
      </button>
      <button type="button" className={clsx('mini-btn', styles.tbtn)} disabled={off} onClick={p.onBack} title="step back (←)" aria-label="Step back">
        ◀
      </button>
      <button
        type="button"
        className={clsx(ui.apply, styles.play)}
        disabled={off}
        onClick={p.onPlay}
        title="play / pause (space)"
        aria-label={p.playing ? 'Pause' : 'Play'}
      >
        {p.playing ? '⏸' : '▶'}
      </button>
      <button type="button" className={clsx('mini-btn', styles.tbtn)} disabled={off} onClick={p.onIn} title="step in (↓ / F11)">
        ⤵ in
      </button>
      <button type="button" className={clsx('mini-btn', styles.tbtn)} disabled={off} onClick={p.onOver} title="step over (→ / F10)">
        ↪ over
      </button>
      <button type="button" className={clsx('mini-btn', styles.tbtn)} disabled={off} onClick={p.onOut} title="step out (↑ / shift+F11)">
        ⤴ out
      </button>
      <button
        type="button"
        className={clsx('mini-btn', styles.tbtn)}
        disabled={off}
        onClick={p.onContinue}
        title="continue to the next breakpoint (F5 / C)"
        aria-label="Continue to the next breakpoint"
      >
        ▶▶
      </button>
      <button type="button" className={clsx('mini-btn', styles.tbtn)} disabled={off} onClick={p.onEnd} title="jump to the end (End)" aria-label="Jump to the end">
        ⏭
      </button>

      <input
        type="range"
        className={styles.scrubber}
        min={0}
        max={Math.max(1, p.len - 1)}
        value={p.idx}
        disabled={off}
        onChange={(e) => p.onJump(Number(e.target.value))}
        aria-label="Step"
        aria-valuetext={`step ${p.len ? p.idx + 1 : 0} of ${p.len}`}
      />
      <span className={styles.counter}>
        {p.len ? p.idx + 1 : 0} / {p.len}
      </span>

      <label className={styles.speed}>
        speed
        <input
          type="range"
          min={0}
          max={SPEED_STEPS}
          value={speedToSlider(p.speed)}
          onChange={(e) => p.onSpeed(sliderToSpeed(Number(e.target.value)))}
          aria-valuetext={`${p.speed.toFixed(2)} times`}
        />
      </label>
      <span className={styles.speedValue}>{p.speed.toFixed(2)}x</span>
      <label className="check">
        <input type="checkbox" checked={p.animate} onChange={(e) => p.onAnimate(e.target.checked)} />
        animate
      </label>
      <button
        type="button"
        className="mini-btn"
        aria-pressed={p.consoleOpen}
        onClick={p.onConsole}
        title="Debug console: log output and call/return events up to this step"
      >
        ⌨ console
      </button>
    </div>
  )
})
