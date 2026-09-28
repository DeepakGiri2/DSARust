// One hook per endpoint. Pages call these and never `fetch` directly, so cache
// keys, invalidation and optimistic updates live in one place.
//
// Profile-scoped hooks take the profile id explicitly and stay idle while it is
// null (a guest, or a signed-in user who has not picked a profile yet).

import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
  type QueryClient,
} from '@tanstack/react-query'
import { api, seg } from './client'
import type {
  AdminOverview,
  AiStatus,
  Catalog,
  ChangePasswordRequest,
  CheckoutRequest,
  CreatePlaylistRequest,
  DeleteAccountRequest,
  DraftMap,
  Guide,
  ImportRequest,
  ImportResult,
  Meta,
  Page,
  Playlist,
  Problem,
  Profile,
  ProfileInput,
  ProfileSettings,
  ProgressEntry,
  ProgressSnapshot,
  ProgressStatus,
  RedirectUrl,
  RunRequest,
  RunResult,
  SessionRow,
  Stats,
  Submission,
  SubmissionSummary,
  TraceRequest,
  TraceResponse,
  UpdateMeRequest,
  User,
  Uuid,
} from './types'

// ─────────────────────────────────────────────────────────────────────────────
// Keys
// ─────────────────────────────────────────────────────────────────────────────

export const qk = {
  meta: ['meta'] as const,
  catalog: ['catalog'] as const,
  guide: ['guide'] as const,
  problem: (slug: string, authed: boolean) => ['problem', slug, authed] as const,
  defaultTrace: (slug: string, authed: boolean) => ['trace', slug, 'default', authed] as const,
  me: ['me'] as const,
  sessions: ['auth', 'sessions'] as const,
  profiles: ['profiles'] as const,
  profile: (pid: Uuid) => ['profiles', pid] as const,
  settings: (pid: Uuid) => ['profiles', pid, 'settings'] as const,
  progress: (pid: Uuid) => ['profiles', pid, 'progress'] as const,
  playlists: (pid: Uuid) => ['profiles', pid, 'playlists'] as const,
  drafts: (pid: Uuid, slug: string) => ['profiles', pid, 'drafts', slug] as const,
  submissions: (pid: Uuid, slug?: string) => ['profiles', pid, 'submissions', slug ?? '*'] as const,
  submission: (pid: Uuid, id: Uuid) => ['profiles', pid, 'submission', id] as const,
  stats: (pid: Uuid) => ['profiles', pid, 'stats'] as const,
  aiStatus: ['ai', 'status'] as const,
  admin: ['admin', 'overview'] as const,
}

const HOUR = 60 * 60 * 1000

// ─────────────────────────────────────────────────────────────────────────────
// Public content (CDN-cached; effectively static between deploys)
// ─────────────────────────────────────────────────────────────────────────────

export function useMeta() {
  return useQuery({ queryKey: qk.meta, queryFn: () => api.get<Meta>('/meta'), staleTime: HOUR })
}

export function useCatalog() {
  return useQuery({
    queryKey: qk.catalog,
    queryFn: () => api.get<Catalog>('/content/catalog'),
    staleTime: HOUR,
  })
}

export function useGuide(enabled = true) {
  return useQuery({
    queryKey: qk.guide,
    queryFn: () => api.get<Guide>('/content/guide'),
    staleTime: HOUR,
    enabled,
  })
}

/**
 * The problem page's content. Signed-in users go through the auth-aware
 * endpoint so premium sources arrive when they are entitled; guests use the
 * public, CDN-cached one.
 */
export function useProblem(slug: string | undefined, authed: boolean) {
  return useQuery({
    queryKey: qk.problem(slug ?? '', authed),
    queryFn: () =>
      api.get<Problem>(authed ? `/problems/${seg(slug!)}` : `/content/problems/${seg(slug!)}`),
    enabled: !!slug,
    staleTime: HOUR,
  })
}

/** The trace for the problem's default input. */
export function useDefaultTrace(slug: string | undefined, opts: { authed: boolean; enabled: boolean }) {
  return useQuery({
    queryKey: qk.defaultTrace(slug ?? '', opts.authed),
    queryFn: () =>
      opts.authed
        ? api.post<TraceResponse>(`/problems/${seg(slug!)}/trace`, {})
        : api.get<TraceResponse>(`/content/problems/${seg(slug!)}/trace`),
    enabled: !!slug && opts.enabled,
    staleTime: HOUR,
    retry: false,
  })
}

/** Re-trace with edited input. Rejects with a 422 `ApiError` whose `.errors` lists every violation. */
export function useTraceMutation(slug: string) {
  return useMutation({
    mutationFn: (req: TraceRequest) => api.post<TraceResponse>(`/problems/${seg(slug)}/trace`, req),
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// Account
// ─────────────────────────────────────────────────────────────────────────────

export function useMe(enabled = true) {
  return useQuery({ queryKey: qk.me, queryFn: () => api.get<User>('/me'), enabled })
}

export function useUpdateMe() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (req: UpdateMeRequest) => api.patch<User>('/me', req),
    onSuccess: (user) => qc.setQueryData(qk.me, user),
  })
}

export function useChangePassword() {
  return useMutation({
    mutationFn: (req: ChangePasswordRequest) => api.post<void>('/auth/password/change', req),
  })
}

export function useDeleteAccount() {
  return useMutation({ mutationFn: (req: DeleteAccountRequest) => api.del<void>('/me', req) })
}

export function useSessions(enabled = true) {
  return useQuery({
    queryKey: qk.sessions,
    queryFn: () => api.get<SessionRow[]>('/auth/sessions'),
    enabled,
  })
}

export function useRevokeSession() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.del<void>(`/auth/sessions/${seg(id)}`),
    onSettled: () => qc.invalidateQueries({ queryKey: qk.sessions }),
  })
}

export function useRevokeOtherSessions() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.post<void>('/auth/sessions/revoke-others'),
    onSettled: () => qc.invalidateQueries({ queryKey: qk.sessions }),
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// Profiles
// ─────────────────────────────────────────────────────────────────────────────

export function useProfiles(enabled = true) {
  return useQuery({
    queryKey: qk.profiles,
    queryFn: () => api.get<Profile[]>('/profiles'),
    enabled,
  })
}

export function useCreateProfile() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (req: ProfileInput) => api.post<Profile>('/profiles', req),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.profiles }),
  })
}

export function useUpdateProfile() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...req }: Partial<ProfileInput> & { id: Uuid }) =>
      api.patch<Profile>(`/profiles/${seg(id)}`, req),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.profiles }),
  })
}

export function useDeleteProfile() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.del<void>(`/profiles/${seg(id)}`),
    onSuccess: (_d, id) => {
      qc.removeQueries({ queryKey: qk.profile(id) })
      return qc.invalidateQueries({ queryKey: qk.profiles })
    },
  })
}

export function useProfileSettings(pid: Uuid | null) {
  return useQuery({
    queryKey: qk.settings(pid ?? ''),
    queryFn: () => api.get<ProfileSettings>(`/profiles/${seg(pid!)}/settings`),
    enabled: !!pid,
    staleTime: Infinity,
  })
}

/** Merge-patch: only the keys sent are changed. */
export function useSaveProfileSettings(pid: Uuid | null) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (patch: ProfileSettings) =>
      api.put<ProfileSettings>(`/profiles/${seg(pid!)}/settings`, patch),
    onSuccess: (settings) => qc.setQueryData(qk.settings(pid!), settings),
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// Progress — optimistic, because a tick or a star must feel instant
// ─────────────────────────────────────────────────────────────────────────────

export function useProgress(pid: Uuid | null) {
  return useQuery({
    queryKey: qk.progress(pid ?? ''),
    queryFn: () => api.get<ProgressSnapshot>(`/profiles/${seg(pid!)}/progress`),
    enabled: !!pid,
  })
}

const EMPTY_ENTRY: ProgressEntry = {
  status: 'todo',
  favourite: false,
  attempts: 0,
  solved_at: null,
  updated_at: new Date(0).toISOString(),
}

/** Entry for a slug, `todo` when untouched — mirrors `Store::entry`. */
export function entryOf(snapshot: ProgressSnapshot | undefined, slug: string): ProgressEntry {
  return snapshot?.entries[slug] ?? EMPTY_ENTRY
}

function recount(entries: Record<string, ProgressEntry>) {
  const all = Object.values(entries)
  return {
    solved: all.filter((e) => e.status === 'solved').length,
    attempted: all.filter((e) => e.status === 'attempted').length,
    favourites: all.filter((e) => e.favourite).length,
  }
}

/** Apply a change to one entry in the cached snapshot. Returns the previous snapshot. */
export function patchProgressCache(
  qc: QueryClient,
  pid: Uuid,
  slug: string,
  patch: (e: ProgressEntry) => ProgressEntry,
): ProgressSnapshot | undefined {
  const key = qk.progress(pid)
  const prev = qc.getQueryData<ProgressSnapshot>(key)
  if (prev) {
    const entries = { ...prev.entries, [slug]: patch(entryOf(prev, slug)) }
    qc.setQueryData<ProgressSnapshot>(key, { entries, stats: recount(entries) })
  }
  return prev
}

function useOptimisticProgress<V>(
  pid: Uuid | null,
  send: (slug: string, value: V) => Promise<ProgressEntry>,
  apply: (e: ProgressEntry, value: V) => ProgressEntry,
) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ slug, value }: { slug: string; value: V }) => send(slug, value),
    onMutate: async ({ slug, value }) => {
      if (!pid) return {}
      await qc.cancelQueries({ queryKey: qk.progress(pid) })
      const prev = patchProgressCache(qc, pid, slug, (e) => apply(e, value))
      return { prev }
    },
    onError: (_e, _v, ctx) => {
      if (pid && ctx?.prev) qc.setQueryData(qk.progress(pid), ctx.prev)
    },
    onSuccess: (entry, { slug }) => {
      if (pid) patchProgressCache(qc, pid, slug, () => entry)
    },
    onSettled: () => {
      if (pid) void qc.invalidateQueries({ queryKey: qk.stats(pid) })
    },
  })
}

/** Force a status (the "mark solved" toggle). */
export function useSetStatus(pid: Uuid | null) {
  return useOptimisticProgress<ProgressStatus>(
    pid,
    (slug, status) =>
      api.put<ProgressEntry>(`/profiles/${seg(pid!)}/progress/${seg(slug)}/status`, { status }),
    (e, status) => ({
      ...e,
      status,
      solved_at: status === 'solved' ? new Date().toISOString() : null,
    }),
  )
}

export function useSetFavourite(pid: Uuid | null) {
  return useOptimisticProgress<boolean>(
    pid,
    (slug, favourite) =>
      api.put<ProgressEntry>(`/profiles/${seg(pid!)}/progress/${seg(slug)}/favourite`, {
        favourite,
      }),
    (e, favourite) => ({ ...e, favourite }),
  )
}

export function useStats(pid: Uuid | null) {
  return useQuery({
    queryKey: qk.stats(pid ?? ''),
    queryFn: () => api.get<Stats>(`/profiles/${seg(pid!)}/stats`),
    enabled: !!pid,
  })
}

export function useImportProgress(pid: Uuid | null) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (req: ImportRequest) => api.post<ImportResult>(`/profiles/${seg(pid!)}/import`, req),
    onSuccess: () => {
      if (!pid) return
      void qc.invalidateQueries({ queryKey: qk.progress(pid) })
      void qc.invalidateQueries({ queryKey: qk.playlists(pid) })
      void qc.invalidateQueries({ queryKey: qk.stats(pid) })
    },
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// Playlists
// ─────────────────────────────────────────────────────────────────────────────

export function usePlaylists(pid: Uuid | null) {
  return useQuery({
    queryKey: qk.playlists(pid ?? ''),
    queryFn: () => api.get<Playlist[]>(`/profiles/${seg(pid!)}/playlists`),
    enabled: !!pid,
  })
}

export function useCreatePlaylist(pid: Uuid | null) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (req: CreatePlaylistRequest) =>
      api.post<Playlist>(`/profiles/${seg(pid!)}/playlists`, req),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.playlists(pid ?? '') }),
  })
}

export function useRenamePlaylist(pid: Uuid | null) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, name }: { id: Uuid; name: string }) =>
      api.patch<Playlist>(`/profiles/${seg(pid!)}/playlists/${seg(id)}`, { name }),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.playlists(pid ?? '') }),
  })
}

export function useDeletePlaylist(pid: Uuid | null) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.del<void>(`/profiles/${seg(pid!)}/playlists/${seg(id)}`),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.playlists(pid ?? '') }),
  })
}

/** Add or remove one problem; optimistic, like the desktop's tick-box menu. */
export function useTogglePlaylistItem(pid: Uuid | null) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, slug, member }: { id: Uuid; slug: string; member: boolean }) => {
      const path = `/profiles/${seg(pid!)}/playlists/${seg(id)}/items/${seg(slug)}`
      return member ? api.put<void>(path) : api.del<void>(path)
    },
    onMutate: async ({ id, slug, member }) => {
      if (!pid) return {}
      const key = qk.playlists(pid)
      await qc.cancelQueries({ queryKey: key })
      const prev = qc.getQueryData<Playlist[]>(key)
      if (prev) {
        qc.setQueryData<Playlist[]>(
          key,
          prev.map((p) =>
            p.id !== id
              ? p
              : {
                  ...p,
                  slugs: member
                    ? [...p.slugs.filter((s) => s !== slug), slug]
                    : p.slugs.filter((s) => s !== slug),
                },
          ),
        )
      }
      return { prev }
    },
    onError: (_e, _v, ctx) => {
      if (pid && ctx?.prev) qc.setQueryData(qk.playlists(pid), ctx.prev)
    },
    onSettled: () => qc.invalidateQueries({ queryKey: qk.playlists(pid ?? '') }),
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// Drafts
// ─────────────────────────────────────────────────────────────────────────────

export function useDrafts(pid: Uuid | null, slug: string | undefined) {
  return useQuery({
    queryKey: qk.drafts(pid ?? '', slug ?? ''),
    queryFn: () => api.get<DraftMap>(`/profiles/${seg(pid!)}/drafts/${seg(slug!)}`),
    enabled: !!pid && !!slug,
    staleTime: Infinity,
  })
}

export function useSaveDraft(pid: Uuid | null, slug: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ lang, code }: { lang: string; code: string }) =>
      api.put<void>(`/profiles/${seg(pid!)}/drafts/${seg(slug)}/${seg(lang)}`, { code }),
    onSuccess: (_d, { lang, code }) => {
      if (!pid) return
      qc.setQueryData<DraftMap>(qk.drafts(pid, slug), (m) => ({
        ...(m ?? {}),
        [lang]: { lang, code, updated_at: new Date().toISOString() },
      }))
    },
  })
}

export function useDeleteDraft(pid: Uuid | null, slug: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (lang: string) =>
      api.del<void>(`/profiles/${seg(pid!)}/drafts/${seg(slug)}/${seg(lang)}`),
    onSuccess: (_d, lang) => {
      if (!pid) return
      qc.setQueryData<DraftMap>(qk.drafts(pid, slug), (m) => {
        const next = { ...(m ?? {}) }
        delete next[lang]
        return next
      })
    },
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// Runs and submissions
// ─────────────────────────────────────────────────────────────────────────────

/** Run or test. On success the problem's progress entry is written into the cache. */
export function useRun(pid: Uuid | null) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (req: RunRequest) => api.post<RunResult>(`/profiles/${seg(pid!)}/runs`, req),
    onSuccess: (res) => {
      if (!pid) return
      patchProgressCache(qc, pid, res.slug, () => res.progress)
      void qc.invalidateQueries({ queryKey: ['profiles', pid, 'submissions'] })
      void qc.invalidateQueries({ queryKey: qk.stats(pid) })
    },
  })
}

export function useSubmissions(pid: Uuid | null, slug?: string) {
  return useInfiniteQuery({
    queryKey: qk.submissions(pid ?? '', slug),
    queryFn: ({ pageParam }) => {
      const q = new URLSearchParams({ limit: '20' })
      if (slug) q.set('slug', slug)
      if (pageParam) q.set('cursor', pageParam)
      return api.get<Page<SubmissionSummary>>(`/profiles/${seg(pid!)}/submissions?${q}`)
    },
    initialPageParam: '' as string,
    getNextPageParam: (last) => last.next_cursor ?? undefined,
    enabled: !!pid,
  })
}

export function useSubmission(pid: Uuid | null, id: Uuid | null) {
  return useQuery({
    queryKey: qk.submission(pid ?? '', id ?? ''),
    queryFn: () => api.get<Submission>(`/profiles/${seg(pid!)}/submissions/${seg(id!)}`),
    enabled: !!pid && !!id,
    staleTime: Infinity,
  })
}

// ─────────────────────────────────────────────────────────────────────────────
// AI, billing, admin
// ─────────────────────────────────────────────────────────────────────────────

export function useAiStatus(enabled = true) {
  return useQuery({ queryKey: qk.aiStatus, queryFn: () => api.get<AiStatus>('/ai/status'), enabled })
}

export function useCheckout() {
  return useMutation({
    mutationFn: (req: CheckoutRequest) => api.post<RedirectUrl>('/billing/checkout', req),
    onSuccess: ({ url }) => window.location.assign(url),
  })
}

export function useBillingPortal() {
  return useMutation({
    mutationFn: () => api.post<RedirectUrl>('/billing/portal'),
    onSuccess: ({ url }) => window.location.assign(url),
  })
}

export function useAdminOverview(enabled = true) {
  return useQuery({
    queryKey: qk.admin,
    queryFn: () => api.get<AdminOverview>('/admin/overview'),
    enabled,
  })
}
