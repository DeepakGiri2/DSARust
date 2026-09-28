// DEVELOPMENT ONLY — the fake API behind ./devMock. Content is copied from
// content/problems/{two-sum,invert-binary-tree}; traces are the real engine's
// fixtures. State (progress, drafts, playlists) lives in memory for the tab.

import type {
  Catalog,
  CatalogProblem,
  DraftMap,
  InputField,
  Language,
  Meta,
  Playlist,
  Problem,
  ProblemSource,
  Profile,
  ProgressEntry,
  RunRequest,
  RunResult,
  SessionInfo,
  TestResult,
  TraceResponse,
} from '@/api/types'
import type { Trace } from '@/trace/types'

const NOW = '2026-09-01T12:00:00Z'
const LATENCY_MS = 350

// ── content ─────────────────────────────────────────────────────────────────

/** `dsa_core::code::parse_code` for `//` markers: strip `//@tag`, record where each landed. */
function parse(source: string): Pick<ProblemSource, 'code' | 'tag_lines' | 'line_tags'> {
  const tag_lines: Record<string, number> = {}
  const line_tags: Record<string, string> = {}
  const lines = source.split('\n').map((line, i) => {
    const trimmed = line.trimEnd()
    const at = trimmed.lastIndexOf('//')
    const tag = at < 0 ? '' : trimmed.slice(at + 2)
    if (!tag.startsWith('@') || !/^[\w-]+$/.test(tag.slice(1))) return line
    tag_lines[tag.slice(1)] ??= i + 1
    line_tags[String(i + 1)] = tag.slice(1)
    return line.slice(0, at).trimEnd()
  })
  return { code: lines.join('\n'), tag_lines, line_tags }
}

const TODO: Record<string, string> = {
  go: '    // write your code here\n    panic("todo")\n',
  cpp: '    // write your code here\n    throw runtime_error("todo");\n',
  java: '    // write your code here\n    throw new UnsupportedOperationException("todo");\n',
}

function source(lang: string, marked: string, signature: string, harness: string | null): ProblemSource {
  return { lang, ...parse(marked), starter: `${signature}\n${TODO[lang]}}\n`, harness, synthesized: false }
}

function field(name: string, label: string, type: InputField['type'], help: string | null = null): InputField {
  return { name, label, type, min: null, max: null, min_len: null, max_len: 40, charset: null, sorted: false, unique: false, help }
}

const TWO_SUM_GO = `func twoSum(nums []int, target int) []int {
    seen := map[int]int{} //@init
    for i, x := range nums { //@loop
        need := target - x //@need
        if j, ok := seen[need]; ok { //@check
            return []int{j, i} //@found
        }
        seen[x] = i //@store
    }
    return nil //@none
}
`
const TWO_SUM_CPP = `vector<int> twoSum(vector<int>& nums, int target) {
    unordered_map<int, int> seen; //@init
    for (int i = 0; i < (int)nums.size(); i++) { //@loop
        int need = target - nums[i]; //@need
        if (seen.count(need)) { //@check
            return {seen[need], i}; //@found
        }
        seen[nums[i]] = i; //@store
    }
    return {}; //@none
}
`
const TWO_SUM_JAVA = `static int[] twoSum(int[] nums, int target) {
    Map<Integer, Integer> seen = new HashMap<>(); //@init
    for (int i = 0; i < nums.length; i++) { //@loop
        int need = target - nums[i]; //@need
        if (seen.containsKey(need)) { //@check
            return new int[]{seen.get(need), i}; //@found
        }
        seen.put(nums[i], i); //@store
    }
    return new int[0]; //@none
}
`
const TWO_SUM_GO_HARNESS = `package main

import (
    "bufio"
    "fmt"
    "os"
    "strconv"
    "strings"
)

func twoSum(nums []int, target int) []int {
    seen := map[int]int{}
    for i, x := range nums {
        need := target - x
        if j, ok := seen[need]; ok {
            return []int{j, i}
        }
        seen[x] = i
    }
    return nil
}

func readInts(reader *bufio.Reader) []int {
    line, _ := reader.ReadString('\\n')
    fields := strings.Fields(line)
    nums := make([]int, len(fields))
    for i, f := range fields {
        nums[i], _ = strconv.Atoi(f)
    }
    return nums
}

func main() {
    reader := bufio.NewReader(os.Stdin)
    nums := readInts(reader)
    target := readInts(reader)[0]
    res := twoSum(nums, target)
    fmt.Println(res[0], res[1])
}
`
const TWO_SUM_JAVA_HARNESS = `import java.util.*;
import java.io.*;

class Main {

    static int[] twoSum(int[] nums, int target) {
        Map<Integer, Integer> seen = new HashMap<>();
        for (int i = 0; i < nums.length; i++) {
            int need = target - nums[i];
            if (seen.containsKey(need)) {
                return new int[]{seen.get(need), i};
            }
            seen.put(nums[i], i);
        }
        return new int[0];
    }

    public static void main(String[] args) throws Exception {
        BufferedReader br = new BufferedReader(new InputStreamReader(System.in));
        int[] nums = Arrays.stream(br.readLine().trim().split("\\\\s+")).mapToInt(Integer::parseInt).toArray();
        int target = Integer.parseInt(br.readLine().trim());
        int[] res = twoSum(nums, target);
        System.out.println(res[0] + " " + res[1]);
    }
}
`
const INVERT_GO = `func invertTree(root *TreeNode) *TreeNode {
    if root == nil { //@base
        return nil //@retnil
    }
    left := invertTree(root.Left) //@recL
    right := invertTree(root.Right) //@recR
    root.Left = right //@setL
    root.Right = left //@setR
    return root //@ret
}
`
const INVERT_CPP = `TreeNode* invertTree(TreeNode* root) {
    if (root == nullptr) { //@base
        return nullptr; //@retnil
    }
    TreeNode* left = invertTree(root->left); //@recL
    TreeNode* right = invertTree(root->right); //@recR
    root->left = right; //@setL
    root->right = left; //@setR
    return root; //@ret
}
`

function problems(authed: boolean): Record<string, Problem> {
  const base = {
    tier: '50' as const,
    hints: [],
    related: [],
    guide_topics: [],
    premium: false,
    locked: false,
    has_trace: true,
  }
  return {
    'two-sum': {
      ...base,
      slug: 'two-sum',
      title: 'Two Sum',
      category: 'Arrays & Hashing',
      difficulty: 'Easy',
      description:
        'Given an array of integers nums and an integer target, return the indices of the two numbers that add up to target. Exactly one solution exists.',
      approach:
        'One pass with a hash map. For each element x, check if (target − x) was already seen; if yes we have the pair, otherwise record x → index in the map.',
      complexity: 'O(n) time · O(n) space',
      leetcode_url: 'https://leetcode.com/problems/two-sum/',
      inputs: [field('nums', 'nums', 'int-array'), field('target', 'target', 'int')],
      default_input: { nums: [2, 7, 11, 15, 3], target: 14 },
      default_fields: { nums: '2 7 11 15 3', target: '14' },
      tests: [
        { name: 'case 1', input: { nums: [2, 7, 11, 15, 3], target: 14 }, stdin: '2 7 11 15 3\n14\n', expected: '2 4', edge: false },
        { name: 'case 2', input: { nums: [3, 2, 4], target: 6 }, stdin: '3 2 4\n6\n', expected: '1 2', edge: true },
      ],
      sources: [
        source('go', TWO_SUM_GO, 'func twoSum(nums []int, target int) []int {', TWO_SUM_GO_HARNESS),
        source('cpp', TWO_SUM_CPP, 'vector<int> twoSum(vector<int>& nums, int target) {', null),
        source('java', TWO_SUM_JAVA, 'static int[] twoSum(int[] nums, int target) {', TWO_SUM_JAVA_HARNESS),
      ],
    },
    'invert-binary-tree': {
      ...base,
      slug: 'invert-binary-tree',
      title: 'Invert Binary Tree',
      category: 'Trees',
      difficulty: 'Easy',
      description:
        'Given the root of a binary tree, invert it (mirror it: every left child becomes the right child and vice versa) and return the root. Great problem for watching recursion — use Step In / Step Out!',
      approach:
        'Recursion: to invert a tree, first invert both subtrees, then swap them. The base case is an empty (nil) node. Watch the call stack grow as we Step In, and collapse as calls return.',
      complexity: 'O(n) time · O(h) space (recursion stack)',
      leetcode_url: 'https://leetcode.com/problems/invert-binary-tree/',
      inputs: [field('tree', "tree (level-order, 'n' = null)", 'tree', 'level order, n for a missing child')],
      default_input: { tree: '4 2 7 1 3 6 9' },
      default_fields: { tree: '4 2 7 1 3 6 9' },
      tests: [
        { name: 'case 1', input: { tree: '4 2 7 1 3 6 9' }, stdin: '4 2 7 1 3 6 9\n', expected: '4 7 2 9 6 3 1', edge: false },
        { name: 'case 2', input: { tree: '2 1 3' }, stdin: '2 1 3\n', expected: '2 3 1', edge: false },
      ],
      sources: [
        source('go', INVERT_GO, 'func invertTree(root *TreeNode) *TreeNode {', null),
        source('cpp', INVERT_CPP, 'TreeNode* invertTree(TreeNode* root) {', null),
      ],
    },
    'lru-cache': {
      ...base,
      slug: 'lru-cache',
      title: 'LRU Cache',
      category: 'Linked List',
      difficulty: 'Medium',
      tier: '150',
      description: 'Design a data structure that follows the constraints of a Least Recently Used (LRU) cache.',
      approach: 'A hash map from key to a node of a doubly linked list kept in recency order.',
      complexity: 'O(1) per operation',
      leetcode_url: 'https://leetcode.com/problems/lru-cache/',
      inputs: [],
      default_input: {},
      default_fields: {},
      tests: [],
      premium: true,
      locked: !authed,
      sources: [],
    },
  }
}

const FIXTURES = import.meta.glob<Trace>('/src/viz/fixtures/{two-sum,invert-binary-tree}.json', {
  import: 'default',
})
const traceLoader = (slug: string) => FIXTURES[`/src/viz/fixtures/${slug}.json`]

const LANGUAGES: Language[] = [
  { id: 'go', label: 'Go', ext: 'go', syntax: 'go', comment: '//', order: 0, runnable: true },
  { id: 'cpp', label: 'C++', ext: 'cpp', syntax: 'cpp', comment: '//', order: 1, runnable: true },
  { id: 'java', label: 'Java', ext: 'java', syntax: 'java', comment: '//', order: 2, runnable: true },
  { id: 'python', label: 'Python', ext: 'py', syntax: 'python', comment: '#', order: 3, runnable: true },
]

function catalog(): Catalog {
  const row = (p: Problem): CatalogProblem => ({
    slug: p.slug,
    title: p.title,
    category: p.category,
    difficulty: p.difficulty,
    tier: p.tier,
    leetcode_url: p.leetcode_url,
    viz: p.has_trace,
    langs: p.sources.map((s) => s.lang),
    premium: p.premium,
  })
  const all = Object.values(problems(false))
  return {
    content_version: 'dev',
    tiers: [],
    categories: [...new Set(all.map((p) => p.category))].map((name) => ({
      name,
      problems: all.filter((p) => p.category === name).map(row),
    })),
    languages: LANGUAGES,
    total: all.length,
    animated: all.length,
  }
}

const META: Meta = {
  version: 'dev',
  content_version: 'dev',
  features: {
    signup: true,
    email_verification_required: false,
    oauth: { github: false, google: false },
    billing: false,
    ai: { enabled: true, provider: 'mock', model: 'mock' },
    runner: true,
  },
  plans: [],
  limits: { profiles_per_account: 5, draft_bytes: 65536 },
  avatars: ['🦊'],
  colors: ['#7c6cff'],
}

const PROFILE: Profile = {
  id: 'dev-profile',
  name: 'Dev',
  avatar: '🦊',
  color: '#7c6cff',
  created_at: NOW,
  last_seen_at: NOW,
  stats: { solved: 0, attempted: 0, favourites: 0 },
}

const SESSION: SessionInfo = {
  user: {
    id: 'dev-user',
    email: 'dev@example.test',
    email_verified: true,
    display_name: 'Dev',
    role: 'user',
    plan: 'pro',
    plan_renews_at: null,
    timezone: 'UTC',
    has_password: true,
    oauth_providers: [],
    created_at: NOW,
  },
  csrf_token: 'dev-csrf',
  entitlements: { plan: 'pro', premium_content: true, ai_daily_limit: 50, ai_used_today: 0, runs_per_minute: 30 },
  profiles: [PROFILE],
}

// ── state ───────────────────────────────────────────────────────────────────

const progress: Record<string, ProgressEntry> = {}
const drafts: Record<string, DraftMap> = {}
const playlists: Playlist[] = []

const entry = (slug: string): ProgressEntry =>
  (progress[slug] ??= { status: 'todo', favourite: false, attempts: 0, solved_at: null, updated_at: NOW })

// ── runs ────────────────────────────────────────────────────────────────────

/** The starter's placeholder panics; anything else "works". Enough to exercise every panel. */
function fakeRun(req: RunRequest): RunResult {
  const problem = problems(true)[req.slug]
  const broken = /todo/.test(req.code)
  const compile = req.lang === 'go' ? null : { ok: true, output: '', duration_ms: 420 }
  const e = entry(req.slug)
  e.attempts += 1
  if (e.status === 'todo') e.status = 'attempted'

  if (req.kind === 'run') {
    const test = problem?.tests.find((t) => t.stdin.trim() === (req.stdin ?? '').trim()) ?? problem?.tests[0]
    return {
      id: crypto.randomUUID(),
      slug: req.slug,
      lang: req.lang,
      kind: 'run',
      mode: req.mode,
      status: broken ? 'runtime_error' : 'ok',
      compile,
      stdout: broken ? '' : `${test?.expected ?? ''}\n`,
      stderr: broken ? 'panic: todo\n\ngoroutine 1 [running]:\nmain.main()\n\t/sandbox/main.go:14 +0x25\nexit status 2' : '',
      exit_code: broken ? 2 : 0,
      timed_out: false,
      passed: 0,
      total: 0,
      duration_ms: 812,
      progress: { ...e },
      created_at: new Date().toISOString(),
    }
  }

  const tests: TestResult[] = (problem?.tests ?? []).map((t) => ({
    name: t.name,
    status: broken ? 'crash' : 'pass',
    stdin: t.stdin,
    expected: t.expected,
    actual: broken ? '' : t.expected,
    detail: broken ? 'panic: todo' : '',
    duration_ms: 310,
    edge: t.edge,
  }))
  const passed = tests.filter((t) => t.status === 'pass').length
  const all = tests.length > 0 && passed === tests.length
  if (all) {
    e.status = 'solved'
    e.solved_at = new Date().toISOString()
  }
  return {
    id: crypto.randomUUID(),
    slug: req.slug,
    lang: req.lang,
    kind: 'test',
    mode: req.mode,
    status: all ? 'passed' : 'failed',
    compile,
    tests,
    passed,
    total: tests.length,
    duration_ms: 1240,
    progress: { ...e },
    created_at: new Date().toISOString(),
  }
}

// ── routing ─────────────────────────────────────────────────────────────────

const json = (status: number, body: unknown) =>
  new Response(status === 204 ? null : JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
const fail = (status: number, code: string, message: string, details?: Record<string, unknown>) =>
  json(status, { error: { code, message, details } })

async function traceFor(slug: string, body: unknown): Promise<Response> {
  const load = traceLoader(slug)
  if (!load) return fail(404, 'not_found', 'No trace for this problem.')
  const fields = (body as { fields?: Record<string, string> } | undefined)?.fields
  if (fields) {
    const errors: string[] = []
    for (const tok of (fields.nums ?? '').split(/[\s,]+/).filter(Boolean)) {
      if (!/^-?\d+$/.test(tok)) errors.push(`nums: "${tok}" is not a whole number`)
    }
    if (fields.target !== undefined && !/^-?\d+$/.test(fields.target.trim())) errors.push('target must be a whole number')
    if (errors.length) return fail(422, 'validation', 'The input is not valid.', { errors })
  }
  const trace = await load()
  const p = problems(true)[slug]
  return json(200, { input: p.default_input, trace } satisfies TraceResponse)
}

export async function handle(
  method: string,
  path: string,
  body: unknown,
  authed: boolean,
  latency = LATENCY_MS,
): Promise<Response> {
  if (latency > 0) await new Promise((r) => setTimeout(r, latency))
  const parts = path.split('/').filter(Boolean).map(decodeURIComponent)
  const is = (m: string, ...pattern: string[]) =>
    method === m && parts.length === pattern.length && pattern.every((p, i) => p === '*' || p === parts[i])

  if (is('GET', 'auth', 'session')) return authed ? json(200, SESSION) : fail(401, 'unauthorized', 'Not signed in.')
  if (is('GET', 'meta')) return json(200, META)
  if (is('GET', 'content', 'catalog')) return json(200, catalog())
  if (is('GET', 'content', 'guide')) return json(200, { topics: [], cheatsheet: [], by_category: {} })
  if (is('GET', 'ai', 'status')) return json(200, { enabled: true, provider: 'mock', model: 'mock', daily_limit: 50, used_today: 0 })

  if (is('GET', 'content', 'problems', '*') || is('GET', 'problems', '*')) {
    const p = problems(authed && parts[0] === 'problems')[parts[parts.length - 1]]
    return p ? json(200, p) : fail(404, 'not_found', 'No such problem.')
  }
  if (is('GET', 'content', 'problems', '*', 'trace')) return traceFor(parts[2], undefined)
  if (is('POST', 'problems', '*', 'trace')) return traceFor(parts[1], body)

  if (!authed) return fail(401, 'unauthorized', 'Not signed in.')
  if (is('GET', 'profiles')) return json(200, [PROFILE])
  if (is('GET', 'profiles', '*', 'settings')) return json(200, {})
  if (is('PUT', 'profiles', '*', 'settings')) return json(200, body)
  if (is('GET', 'profiles', '*', 'progress')) {
    const all = Object.values(progress)
    return json(200, {
      entries: progress,
      stats: {
        solved: all.filter((e) => e.status === 'solved').length,
        attempted: all.filter((e) => e.status === 'attempted').length,
        favourites: all.filter((e) => e.favourite).length,
      },
    })
  }
  if (is('PUT', 'profiles', '*', 'progress', '*', 'status')) {
    const e = entry(parts[3])
    e.status = (body as { status: ProgressEntry['status'] }).status
    e.solved_at = e.status === 'solved' ? new Date().toISOString() : null
    return json(200, e)
  }
  if (is('PUT', 'profiles', '*', 'progress', '*', 'favourite')) {
    const e = entry(parts[3])
    e.favourite = (body as { favourite: boolean }).favourite
    return json(200, e)
  }
  if (is('GET', 'profiles', '*', 'playlists')) return json(200, playlists)
  if (is('POST', 'profiles', '*', 'playlists')) {
    const req = body as { name: string; slugs?: string[] }
    const pl: Playlist = { id: crypto.randomUUID(), name: req.name, slugs: req.slugs ?? [], created_at: NOW }
    playlists.push(pl)
    return json(201, pl)
  }
  if (is('PUT', 'profiles', '*', 'playlists', '*', 'items', '*') || is('DELETE', 'profiles', '*', 'playlists', '*', 'items', '*')) {
    const pl = playlists.find((l) => l.id === parts[3])
    if (!pl) return fail(404, 'not_found', 'No such playlist.')
    pl.slugs = pl.slugs.filter((s) => s !== parts[5])
    if (method === 'PUT') pl.slugs.push(parts[5])
    return json(204, null)
  }
  if (is('GET', 'profiles', '*', 'drafts', '*')) return json(200, drafts[parts[3]] ?? {})
  if (is('PUT', 'profiles', '*', 'drafts', '*', '*')) {
    const [slug, lang] = [parts[3], parts[4]]
    drafts[slug] = { ...drafts[slug], [lang]: { lang, code: (body as { code: string }).code, updated_at: new Date().toISOString() } }
    return json(204, null)
  }
  if (is('DELETE', 'profiles', '*', 'drafts', '*', '*')) {
    delete drafts[parts[3]]?.[parts[4]]
    return json(204, null)
  }
  if (is('POST', 'profiles', '*', 'runs')) {
    if (latency > 0) await new Promise((r) => setTimeout(r, 2 * latency))
    return json(200, fakeRun(body as RunRequest))
  }
  return fail(404, 'not_found', `Mock API has no ${method} ${path}`)
}
