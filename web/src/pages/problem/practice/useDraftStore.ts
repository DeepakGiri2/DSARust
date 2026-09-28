// Editor drafts, autosaved per (problem, language).
//
// Guests keep theirs in localStorage. Signed-in users save to their account,
// debounced — but every edit is also written to the device at once, and that
// copy is only dropped after the server has acknowledged the same text. A
// request fired from `pagehide` is routinely cancelled by the browser, so
// without the device copy the last second of typing before closing the tab
// would simply be gone; with it, the next visit restores and re-syncs it.

import { useCallback, useEffect, useRef } from 'react'
import { useDeleteDraft, useDrafts, useSaveDraft } from '@/api/hooks'
import type { Uuid } from '@/api/types'
import { useToast } from '@/ui'
import { readJson, writeStore } from '../shared/storage'

const SAVE_DEBOUNCE_MS = 800

interface DeviceDraft {
  code: string
  /** Epoch ms of the edit. */
  at: number
}

const isDeviceDraft = (v: unknown): v is DeviceDraft =>
  typeof v === 'object' &&
  v !== null &&
  typeof (v as DeviceDraft).code === 'string' &&
  typeof (v as DeviceDraft).at === 'number'

export const draftKey = (owner: string, slug: string, lang: string) => `dsa.draft.${owner}.${slug}.${lang}`

export interface DraftStore {
  /** False while a signed-in user's drafts are still loading — the editor waits rather than flash the starter. */
  ready: boolean
  /** The saved code for a language, or undefined to start from the starter. Stable once ready. */
  initial: (lang: string) => string | undefined
  /** Record an edit. */
  save: (lang: string, code: string) => void
  /** "reset code": forget the draft everywhere. */
  discard: (lang: string) => void
}

export function useDraftStore({
  pid,
  slug,
  langs,
  limit,
}: {
  pid: Uuid | null
  slug: string
  langs: readonly string[]
  /** `Meta.limits.draft_bytes`, when known. */
  limit: number | undefined
}): DraftStore {
  const owner = pid ?? 'guest'
  const server = useDrafts(pid, slug)
  const saveDraft = useSaveDraft(pid, slug)
  const deleteDraft = useDeleteDraft(pid, slug)
  const toast = useToast()
  const ready = pid === null || !server.isPending

  const pending = useRef(new Map<string, string>())
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const warned = useRef({ failed: false, tooBig: false, load: false })
  const resolved = useRef(new Map<string, string | undefined>())

  // Unload handlers and timers run outside render; they read the latest here.
  const live = useRef({ pid, limit, saveDraft, toast })
  useEffect(() => {
    live.current = { pid, limit, saveDraft, toast }
  })

  const flush = useCallback(() => {
    if (timer.current) {
      clearTimeout(timer.current)
      timer.current = null
    }
    const { pid: p, limit: max, saveDraft: m, toast: t } = live.current
    const batch = [...pending.current]
    pending.current.clear()
    if (!p) return
    for (const [lang, code] of batch) {
      if (max !== undefined && code.length > max) {
        if (!warned.current.tooBig) {
          warned.current.tooBig = true
          t.error(
            `This code is over the ${max.toLocaleString()}-character limit for saved drafts, so it is only kept on this device.`,
          )
        }
        continue
      }
      m.mutateAsync({ lang, code }).then(
        () => {
          const key = draftKey(p, slug, lang)
          if (readJson('local', key, isDeviceDraft)?.code === code) writeStore('local', key, null)
        },
        () => {
          if (!warned.current.failed) {
            warned.current.failed = true
            t.error('Could not save your code to your account. It is kept on this device and will sync on your next visit.')
          }
        },
      )
    }
  }, [slug])

  // Hidden tab, closed tab, left the page: send what is waiting.
  useEffect(() => {
    const onVisibility = () => document.visibilityState === 'hidden' && flush()
    document.addEventListener('visibilitychange', onVisibility)
    window.addEventListener('pagehide', flush)
    return () => {
      document.removeEventListener('visibilitychange', onVisibility)
      window.removeEventListener('pagehide', flush)
      flush()
    }
  }, [flush])

  // Once the account's drafts are in: push device copies the server never got,
  // drop ones the account has since moved past (edited on another device).
  const reconciled = useRef(false)
  useEffect(() => {
    if (!ready || !pid || reconciled.current) return
    reconciled.current = true
    if (server.isError && !warned.current.load) {
      warned.current.load = true
      toast.error('Could not load your saved code. Anything you type now is still saved.')
    }
    for (const lang of langs) {
      const key = draftKey(pid, slug, lang)
      const device = readJson('local', key, isDeviceDraft)
      if (!device) continue
      const saved = server.data?.[lang]
      if (!saved || device.at >= Date.parse(saved.updated_at)) {
        if (device.code !== saved?.code) pending.current.set(lang, device.code)
        else writeStore('local', key, null)
      } else {
        writeStore('local', key, null)
      }
    }
    if (pending.current.size > 0) flush()
  }, [ready, pid, slug, langs, server.isError, server.data, toast, flush])

  const initial = (lang: string): string | undefined => {
    if (!ready) return undefined
    if (resolved.current.has(lang)) return resolved.current.get(lang)
    const device = readJson('local', draftKey(owner, slug, lang), isDeviceDraft)
    const saved = pid ? server.data?.[lang] : undefined
    const code =
      device && (!saved || device.at >= Date.parse(saved.updated_at)) ? device.code : saved?.code
    resolved.current.set(lang, code)
    return code
  }

  const save = (lang: string, code: string) => {
    writeStore('local', draftKey(owner, slug, lang), JSON.stringify({ code, at: Date.now() } satisfies DeviceDraft))
    if (!pid) return
    pending.current.set(lang, code)
    if (timer.current) clearTimeout(timer.current)
    timer.current = setTimeout(flush, SAVE_DEBOUNCE_MS)
  }

  const discard = (lang: string) => {
    pending.current.delete(lang)
    resolved.current.set(lang, undefined)
    writeStore('local', draftKey(owner, slug, lang), null)
    if (pid) {
      deleteDraft.mutate(lang, {
        onError: () => toast.error('Could not delete the saved copy of this code from your account.'),
      })
    }
  }

  return { ready, initial, save, discard }
}
