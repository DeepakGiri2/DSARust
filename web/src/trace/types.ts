// The trace model, exactly as `dsa-core` (crates/dsa-core/src/model.rs)
// serializes it. The API returns these shapes verbatim from the Rhai engine, so
// this file is a mirror, not a design: when model.rs changes, this changes.
//
// Serde conventions that shape these types:
//  * `Option<T>` fields marked `skip_serializing_if = "Option::is_none"` are
//    *omitted* when empty → `field?: T`.
//  * `Option<T>` fields without that attribute serialize as `null` → `T | null`.
//  * `Vec` fields marked `skip_serializing_if = "Vec::is_empty"` are omitted
//    when empty → `field?: T[]` (treat missing as []).
//  * Rust tuples serialize as fixed-length arrays → `[a, b]`.
//  * `VarMap` serializes as a JSON object in insertion order.

/** A scalar inside a container view. `#[serde(untagged)]` over f64 | String. */
export type Cell = number | string

// ── variables ───────────────────────────────────────────────────────────────

/** `VarVal`, `#[serde(tag = "kind", rename_all = "lowercase")]`. */
export type VarVal =
  | { kind: 'num'; v: Cell }
  | { kind: 'str'; v: string }
  | { kind: 'bool'; v: boolean }
  | { kind: 'arr'; v: Cell[]; hl?: number[] }
  | { kind: 'map'; v: [string, Cell][]; hl?: string[] }
  | { kind: 'set'; v: Cell[]; hl?: Cell[] }
  | { kind: 'ptr'; v: string }
  | { kind: 'null' }

/** Insertion-ordered `name -> value`. Iterate with `Object.entries`. */
export type VarMap = Record<string, VarVal>

/** One call-stack frame. `fn` is a display label, e.g. `climb(n=4)`. */
export interface Frame {
  fn: string
  vars: VarMap
}

// ── views ───────────────────────────────────────────────────────────────────

/** `name -> index` cursor, ordered so a pointer keeps its colour and lane. */
export type Pointer = [name: string, index: number]

export interface ArrayView {
  type: 'array'
  label: string
  data: Cell[]
  pointers?: Pointer[]
  /** Inclusive `[lo, hi]` highlight band — the sliding window. */
  window?: [number, number]
  hl?: number[]
  bad?: number[]
  done?: number[]
  /** Render as a bar chart with heights taken from the values. */
  bars?: boolean
}

export interface KvView {
  type: 'kv'
  label: string
  entries: [key: string, value: Cell][]
  hlKeys?: string[]
  badKeys?: string[]
}

export type StackKind = 'stack' | 'queue' | 'deque' | 'heap'

export interface StackView {
  type: 'stack'
  label: string
  items: Cell[]
  /** Always serialized (no skip attribute); defaults to 'stack'. */
  kind: StackKind
  /** The top item was just pushed — flashes green and slides in. */
  pushed?: boolean
  /** An item was just removed — drawn fading out past the open end. */
  popped?: Cell
  bad?: boolean
}

export interface ListNode {
  id: number
  val: Cell
  next: number | null
}

export interface LinkedListView {
  type: 'list'
  label: string
  nodes: ListNode[]
  /** `name -> node id`, `null` meaning nil. */
  pointers?: [name: string, id: number | null][]
  /** Ids whose outgoing arrow now points backwards (drawn flipped/green). */
  reversed?: number[]
}

export interface TreeNode {
  id: number
  val: Cell
  left: number | null
  right: number | null
}

export interface TreeView {
  type: 'tree'
  label: string
  nodes: TreeNode[]
  root: number | null
  cur?: number
  done?: number[]
  path?: number[]
  /** A pair whose subtrees were just swapped — animated along an arc. */
  swap?: [number, number]
}

/** `(row, col)`. */
export type RC = [row: number, col: number]

export interface GridView {
  type: 'grid'
  label: string
  data: Cell[][]
  hl?: RC[]
  bad?: RC[]
  done?: RC[]
  path?: RC[]
  cur?: RC
  rowLabels?: string[]
  colLabels?: string[]
}

export interface GraphNode {
  id: number
  label: string
  /** Optional layout hint in unit space (0..1). Absent → circular layout. */
  x?: number
  y?: number
}

export interface GraphEdge {
  from: number
  to: number
  directed?: boolean
  weight?: Cell
}

export interface GraphView {
  type: 'graph'
  label: string
  nodes: GraphNode[]
  edges: GraphEdge[]
  cur?: number
  done?: number[]
  /** Discovered but not yet processed — the BFS/DFS frontier. */
  frontier?: number[]
  bad?: number[]
  /** `(from, to)` pairs being traversed — the pulse runs along them. */
  active?: [number, number][]
}

export interface BitRow {
  label: string
  value: number
  /** Always serialized; defaults to 32. */
  width: number
  /** Bit positions to flash, counted from the least-significant bit. */
  hl?: number[]
}

export interface BitsView {
  type: 'bits'
  label: string
  rows: BitRow[]
}

/** Free-form annotation panel: derivations, invariants, running formulas. */
export interface TextView {
  type: 'text'
  label: string
  lines: string[]
  hl?: number[]
}

/** `VizView`, `#[serde(tag = "type", rename_all = "lowercase")]`. */
export type VizView =
  | ArrayView
  | KvView
  | StackView
  | LinkedListView
  | TreeView
  | GridView
  | GraphView
  | BitsView
  | TextView

export type VizKind = VizView['type']

// ── steps and traces ────────────────────────────────────────────────────────

export type StepEvent = 'stmt' | 'call' | 'return'
export type LogKind = 'log' | 'call' | 'return' | 'result'

export interface LogEntry {
  /** Index of the step this line was emitted during. */
  step: number
  text: string
  kind: LogKind
}

/** One recorded step of execution. */
export interface Step {
  /** `//@tag` marker naming the source line, resolved per language. */
  tag: string
  /** Call depth — what makes step-over / step-out possible. */
  depth: number
  event: StepEvent
  /** Call stack snapshot, innermost **last**. */
  frames: Frame[]
  views: VizView[]
  note: string
  /** Log lines emitted up to and including this step (slice of `Trace.logs`). */
  log_len: number
}

/** A complete recorded execution. */
export interface Trace {
  steps: Step[]
  logs: LogEntry[]
  /** Value the traced function returned, for the result banner. */
  result?: string
}

/** Log lines visible at step `i` — mirrors `Trace::logs_at`. */
export function logsAt(trace: Trace, i: number): LogEntry[] {
  const n = trace.steps[i]?.log_len ?? 0
  return trace.logs.slice(0, Math.min(n, trace.logs.length))
}

/** Highest call depth reached — mirrors `Trace::max_depth`. */
export function maxDepth(trace: Trace): number {
  return trace.steps.reduce((m, s) => Math.max(m, s.depth), 0)
}

/** Stable display form of a cell — mirrors `impl Display for Cell`. */
export function cellText(c: Cell): string {
  if (typeof c === 'number') {
    return Number.isInteger(c) && Math.abs(c) < 1e15 ? String(Math.trunc(c)) : String(c)
  }
  return c
}

/** Cell equality bridging number and string — mirrors `impl PartialEq for Cell`. */
export function cellEq(a: Cell, b: Cell): boolean {
  if (typeof a === typeof b) return a === b
  return cellText(a) === cellText(b)
}

/** One-line summary for a collapsed variable — mirrors `VarVal::summary`. */
export function varSummary(v: VarVal): string {
  switch (v.kind) {
    case 'num':
      return cellText(v.v)
    case 'str':
      return `"${v.v}"`
    case 'bool':
      return String(v.v)
    case 'arr':
      return `[${v.v.map(cellText).join(', ')}]`
    case 'map':
      return `{${v.v.length} entries}`
    case 'set':
      return `{${v.v.length} items}`
    case 'ptr':
      return v.v
    case 'null':
      return 'nil'
  }
}

/** True when the variables panel should offer an expandable child list. */
export function varIsComposite(v: VarVal): boolean {
  return v.kind === 'map' || v.kind === 'set' || v.kind === 'arr'
}
