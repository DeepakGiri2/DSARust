// The ⏵ Visualize tab — crates/dsa-app/src/problem.rs.
//
// Code and input on the left, the note bar and canvas in the middle,
// variables / call stack / logs on the right, the transport bar along the
// bottom. `useDebugger` drives the timeline and publishes a snapshot per
// animation frame while something moves; every panel except the canvas is
// memoised on the step, so a transition repaints the canvas and nothing else.

import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { Link } from 'react-router'
import clsx from 'clsx'
import { errorMessage, isApiError } from '@/api/client'
import { useDefaultTrace, useTraceMutation } from '@/api/hooks'
import type { Problem, ProblemSource } from '@/api/types'
import { useDebugger } from '@/debugger/useDebugger'
import { highlightLines } from '@/features/editor'
import { useSettings } from '@/state/settings'
import { logsAt, type Trace } from '@/trace/types'
import { PageSpinner, Seg, type SegOption } from '@/ui'
import { VizCanvas } from '@/viz'
import type { LangInfo } from '../shared/langs'
import { Splitter, useColumnWidth, type ColumnSpec } from '../shared/Splitter'
import { readStore, writeStore } from '../shared/storage'
import ui from '../shared/ui.module.css'
import { COMPACT_QUERY, PHONE_QUERY, useMediaQuery } from '../shared/useMediaQuery'
import { CodePanel } from './CodePanel'
import { InputPanel } from './InputPanel'
import { Inspector } from './Inspector'
import { keyAction, type DebugAction } from './keys'
import { Console } from './Logs'
import { About, Gate, NoteBar } from './parts'
import { Transport } from './Transport'
import styles from './Visualize.module.css'

const CODE: ColumnSpec = { id: 'visualize.code', initial: 430, min: 300, max: 760 }
const INSPECT: ColumnSpec = { id: 'visualize.inspect', initial: 320, min: 240, max: 520 }
const SIDE: ColumnSpec = { id: 'visualize.side', initial: 380, min: 280, max: 560 }

type Pane = 'canvas' | 'code' | 'inspect'

/** The reveal is remembered for the browser session, per problem. */
export const revealKey = (slug: string) => `dsa.revealed.${slug}`

function traceErrors(e: unknown): string[] {
  if (isApiError(e, 'validation')) return e.errors.length ? e.errors : [e.message]
  if (isApiError(e, 'rate_limited')) {
    return [`Too many re-visualizations in a row — try again in ${e.retryAfterSecs ?? 10}s.`]
  }
  return [errorMessage(e)]
}

export interface VisualizeTabProps {
  problem: Problem
  source: ProblemSource
  lang: LangInfo
  authed: boolean
  /** A solved problem skips the gate: there is nothing left to spoil. */
  solved: boolean
  hidden: boolean
  practiceHref: string
  onPractice: () => void
}

export function VisualizeTab({ problem, source, lang, authed, solved, hidden, practiceHref, onPractice }: VisualizeTabProps) {
  const { settings, update, isWatched, toggleWatch } = useSettings()
  const slug = problem.slug

  const [revealed, setRevealed] = useState(() => readStore('session', revealKey(slug)) === '1')
  const open = revealed || solved
  const reveal = () => {
    writeStore('session', revealKey(slug), '1')
    setRevealed(true)
  }

  // ── trace ─────────────────────────────────────────────────────────────────
  // Fetched as soon as the tab is first opened, gate or not, so the reveal is
  // instant. A re-visualized trace replaces the default one.
  const defaultTrace = useDefaultTrace(slug, { authed, enabled: problem.has_trace && !problem.locked })
  const retrace = useTraceMutation(slug)
  const [custom, setCustom] = useState<Trace | null>(null)
  const [inputErrors, setInputErrors] = useState<string[]>([])
  const trace = custom ?? defaultTrace.data?.trace ?? null

  const dbg = useDebugger(trace, { resetKey: slug, speed: settings.speed, animate: settings.animate })
  const { pause, toggleBreakpointTag, clearBreakpoints } = dbg

  const { mutate: traceWith } = retrace
  const applyInput = useCallback(
    (fields: Record<string, string>) =>
      traceWith(
        { fields },
        {
          onSuccess: (res) => {
            setCustom(res.trace)
            setInputErrors([])
          },
          onError: (e) => setInputErrors(traceErrors(e)),
        },
      ),
    [traceWith],
  )

  // ── breakpoints ───────────────────────────────────────────────────────────
  // Set on source *tags*, like the desktop, and kept as tags: the timeline
  // stores step indices, which mean nothing in a re-recorded trace. So each
  // new trace (edited input) gets the same tags re-applied to its own steps,
  // and the breakpoints survive a language switch — the tag just lands on
  // that language's line.
  const [bpTags, setBpTags] = useState<ReadonlySet<string>>(() => new Set())
  const bpRef = useRef(bpTags)
  useLayoutEffect(() => {
    bpRef.current = bpTags
  })
  const toggleBreakpoint = useCallback(
    (tag: string) => {
      setBpTags((prev) => {
        const next = new Set(prev)
        if (!next.delete(tag)) next.add(tag)
        return next
      })
      toggleBreakpointTag(tag)
    },
    [toggleBreakpointTag],
  )
  // Runs after useDebugger's own effect has pointed the timeline at `trace`.
  useEffect(() => {
    if (!trace) return
    clearBreakpoints()
    bpRef.current.forEach((tag) => toggleBreakpointTag(tag))
  }, [trace, clearBreakpoints, toggleBreakpointTag])

  // ── keyboard ──────────────────────────────────────────────────────────────
  const actions = useMemo<Record<DebugAction, () => void>>(
    () => ({
      back: dbg.stepBack,
      over: dbg.stepOver,
      in: dbg.stepIn,
      out: dbg.stepOut,
      play: dbg.togglePlay,
      restart: dbg.restart,
      continue: dbg.continueRun,
      end: dbg.toEnd,
    }),
    [dbg.stepBack, dbg.stepOver, dbg.stepIn, dbg.stepOut, dbg.togglePlay, dbg.restart, dbg.continueRun, dbg.toEnd],
  )
  useEffect(() => {
    if (hidden || !open || !trace) return
    const onKey = (e: KeyboardEvent) => {
      // The helper (or any dialog) owns the keyboard while it is open.
      if (e.defaultPrevented || document.querySelector('[aria-modal="true"]')) return
      const action = keyAction(e)
      if (!action) return
      e.preventDefault()
      actions[action]()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [hidden, open, trace, actions])

  // Playback stops when the tab is put away, as it would with the window closed.
  useEffect(() => {
    if (hidden) pause()
  }, [hidden, pause])

  // ── the current step ──────────────────────────────────────────────────────
  const step = dbg.step
  const prevStep = (dbg.idx > 0 && trace?.steps[dbg.idx - 1]) || null
  // A clicked frame is pinned only for the step it was clicked on; moving on
  // shows the innermost frame again (`frame_sel = None` on the desktop).
  const [frameSel, setFrameSel] = useState<{ idx: number; frame: number } | null>(null)
  const frames = step?.frames.length ?? 0
  const frameIdx =
    frameSel && frameSel.idx === dbg.idx && frameSel.frame < frames ? frameSel.frame : Math.max(0, frames - 1)
  const { idx } = dbg
  const selectFrame = useCallback((frame: number) => setFrameSel({ idx, frame }), [idx])

  const lines = useMemo(() => highlightLines(source.code, lang.syntax), [source.code, lang.syntax])
  const currentLine = step ? (source.tag_lines[step.tag] ?? null) : null
  const logs = useMemo(() => (trace ? logsAt(trace, idx) : []), [trace, idx])

  const [consoleOpen, setConsoleOpen] = useState(false)
  const toggleConsole = useCallback(() => setConsoleOpen((o) => !o), [])
  const setSpeed = useCallback((speed: number) => update({ speed }), [update])
  const setAnimate = useCallback((animate: boolean) => update({ animate }), [update])
  const setShowLogs = useCallback((show_logs: boolean) => update({ show_logs }), [update])

  // ── layout ────────────────────────────────────────────────────────────────
  const compact = useMediaQuery(COMPACT_QUERY)
  const phone = useMediaQuery(PHONE_QUERY)
  const [codeWidth, setCodeWidth] = useColumnWidth(CODE)
  const [inspectWidth, setInspectWidth] = useColumnWidth(INSPECT)
  const [sideWidth, setSideWidth] = useColumnWidth(SIDE)
  const [pane, setPane] = useState<Pane>(phone ? 'canvas' : 'code')
  const paneOptions: SegOption<Pane>[] = [
    ...(phone ? [{ value: 'canvas' as const, label: 'walkthrough' }] : []),
    { value: 'code', label: 'code & input' },
    { value: 'inspect', label: 'variables' },
  ]
  const shownPane = paneOptions.some((o) => o.value === pane) ? pane : paneOptions[0].value

  if (!open) {
    return (
      <div className={styles.viz} hidden={hidden}>
        <Gate langLabel={lang.label} onPractice={onPractice} onReveal={reveal} />
      </div>
    )
  }

  const codeColumn = (
    <>
      <CodePanel
        lines={lines}
        lineTags={source.line_tags}
        currentLine={currentLine}
        breakpoints={bpTags}
        onToggleBreakpoint={toggleBreakpoint}
        langLabel={lang.label}
      />
      {problem.has_trace && problem.inputs.length > 0 && (
        <InputPanel
          fields={problem.inputs}
          defaults={problem.default_fields}
          errors={inputErrors}
          busy={retrace.isPending}
          onApply={applyInput}
        />
      )}
    </>
  )

  const inspector = (
    <Inspector
      slug={slug}
      step={step}
      prevStep={prevStep}
      frameIdx={frameIdx}
      onSelectFrame={selectFrame}
      isWatched={isWatched}
      onToggleWatch={toggleWatch}
      logs={logs}
      showLogs={settings.show_logs}
      onShowLogs={setShowLogs}
    />
  )

  const center = (
    <>
      <NoteBar step={step} breakpoint={dbg.breakpoints.has(idx)} />
      <div className={styles.canvasScroll}>
        {!problem.has_trace ? (
          <div className={ui.card}>
            This problem ships code and tests, but no animation yet. <Link to={practiceHref}>Practice it</Link>{' '}
            instead — the reference code is on the left.
          </div>
        ) : !trace && defaultTrace.isError ? (
          <div className={clsx(ui.card, styles.traceError)} role="alert">
            <code>{traceErrors(defaultTrace.error).join('\n')}</code>
            <button type="button" className="mini-btn" onClick={() => void defaultTrace.refetch()}>
              try again
            </button>
          </div>
        ) : !trace || !step ? (
          <PageSpinner label="Recording the walkthrough…" />
        ) : (
          <>
            <VizCanvas
              className={styles.canvas}
              views={step.views}
              prev={dbg.fromStep && dbg.fromStep !== step ? dbg.fromStep.views : null}
              t={dbg.t}
            />
            {dbg.atEnd && trace.result !== undefined && (
              <div className={clsx(ui.card, styles.result)} aria-live="polite">
                <span className={styles.resultLabel}>result</span>
                <code className={styles.resultValue}>{trace.result}</code>
              </div>
            )}
          </>
        )}
        <About problem={problem} />
      </div>
    </>
  )

  const tabs = (
    <Seg small className={ui.paneTabs} value={shownPane} options={paneOptions} onChange={setPane} aria-label="Panel" />
  )

  return (
    <div className={styles.viz} hidden={hidden}>
      {!compact ? (
        <div className={styles.columns}>
          <section key="code" className={clsx(styles.side, styles.codeCol)} style={{ flexBasis: codeWidth }}>
            {codeColumn}
          </section>
          <Splitter key="code-split" spec={CODE} width={codeWidth} onResize={setCodeWidth} side="left" label="Resize the code column" />
          <section key="center" className={styles.center} aria-label="Walkthrough">
            {center}
          </section>
          <Splitter
            key="inspect-split"
            spec={INSPECT}
            width={inspectWidth}
            onResize={setInspectWidth}
            side="right"
            label="Resize the variables column"
          />
          <section key="inspect" className={clsx(styles.side, styles.inspector)} style={{ flexBasis: inspectWidth }} aria-label="Variables">
            {inspector}
          </section>
        </div>
      ) : (
        <div className={clsx(styles.columns, phone && styles.stacked)}>
          <section key="side" className={clsx(styles.side, styles.codeCol)} style={phone ? undefined : { flexBasis: sideWidth }}>
            {tabs}
            <div className={styles.pane} hidden={shownPane !== 'code'}>
              {codeColumn}
            </div>
            <div className={clsx(styles.pane, styles.inspector)} hidden={shownPane !== 'inspect'}>
              {inspector}
            </div>
          </section>
          {!phone && (
            <Splitter key="side-split" spec={SIDE} width={sideWidth} onResize={setSideWidth} side="left" label="Resize the side panel" />
          )}
          <section key="center" className={styles.center} aria-label="Walkthrough" hidden={phone && shownPane !== 'canvas'}>
            {center}
          </section>
        </div>
      )}
      <Transport
        idx={idx}
        len={dbg.len}
        playing={dbg.playing}
        speed={settings.speed}
        animate={settings.animate}
        consoleOpen={consoleOpen}
        onRestart={dbg.restart}
        onBack={dbg.stepBack}
        onPlay={dbg.togglePlay}
        onIn={dbg.stepIn}
        onOver={dbg.stepOver}
        onOut={dbg.stepOut}
        onContinue={dbg.continueRun}
        onEnd={dbg.toEnd}
        onJump={dbg.jumpTo}
        onSpeed={setSpeed}
        onAnimate={setAnimate}
        onConsole={toggleConsole}
      />
      {consoleOpen && <Console entries={logs} step={idx} />}
    </div>
  )
}
