// The desktop's `Settings` (crates/dsa-app/src/settings.rs), per profile.
//
// Reads merge server state over the defaults; writes apply locally at once and
// reach the server as a debounced merge-patch, so dragging the speed slider is
// one request, not sixty. Guests keep theirs in localStorage.

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react'
import { useProfileSettings, useSaveProfileSettings } from '@/api/hooks'
import type { ProfileSettings } from '@/api/types'
import { useActiveProfileId } from './session'

export type Settings = Required<ProfileSettings>

/** Same defaults as the desktop's `impl Default for Settings`. */
export const DEFAULT_SETTINGS: Settings = {
  lang: 'go',
  tier: '150',
  speed: 1,
  animate: true,
  show_logs: true,
  viz_only: false,
  status_filter: 'all',
  favourites_only: false,
  playlist: null,
  show_question: true,
  ai_open: false,
  watched: [],
  backdrop: true,
  theme: 'dark',
}

const GUEST_KEY = 'dsa.guestSettings'
const SAVE_DEBOUNCE_MS = 600

function readGuest(): ProfileSettings {
  try {
    const raw = localStorage.getItem(GUEST_KEY)
    return raw ? (JSON.parse(raw) as ProfileSettings) : {}
  } catch {
    return {}
  }
}

function writeGuest(s: ProfileSettings) {
  try {
    localStorage.setItem(GUEST_KEY, JSON.stringify(s))
  } catch {
    // Storage unavailable: settings last for this tab only.
  }
}

interface SettingsContextValue {
  settings: Settings
  update: (patch: ProfileSettings) => void
  /** Watch pins are namespaced per problem, exactly like the desktop. */
  isWatched: (slug: string, variable: string) => boolean
  toggleWatch: (slug: string, variable: string) => void
}

const SettingsContext = createContext<SettingsContextValue | null>(null)

export const watchKey = (slug: string, variable: string) => `${slug}::${variable}`

export function SettingsProvider({ children }: { children: ReactNode }) {
  const pid = useActiveProfileId()
  const server = useProfileSettings(pid)
  const save = useSaveProfileSettings(pid)

  const [local, setLocal] = useState<ProfileSettings>(() => (pid ? {} : readGuest()))
  const pending = useRef<ProfileSettings>({})
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Switching profile (or signing out) starts from that profile's own state.
  useEffect(() => {
    setLocal(pid ? {} : readGuest())
    pending.current = {}
    if (timer.current) clearTimeout(timer.current)
  }, [pid])

  const flush = useCallback(() => {
    timer.current = null
    const patch = pending.current
    pending.current = {}
    if (pid && Object.keys(patch).length) save.mutate(patch)
  }, [pid, save])

  // Do not lose the last change when the tab closes mid-debounce.
  useEffect(() => {
    const onHide = () => {
      if (timer.current) {
        clearTimeout(timer.current)
        flush()
      }
    }
    window.addEventListener('pagehide', onHide)
    return () => window.removeEventListener('pagehide', onHide)
  }, [flush])

  const update = useCallback(
    (patch: ProfileSettings) => {
      setLocal((prev) => {
        const next = { ...prev, ...patch }
        if (!pid) writeGuest(next)
        return next
      })
      if (pid) {
        pending.current = { ...pending.current, ...patch }
        if (timer.current) clearTimeout(timer.current)
        timer.current = setTimeout(flush, SAVE_DEBOUNCE_MS)
      }
    },
    [pid, flush],
  )

  const settings = useMemo<Settings>(
    () => ({ ...DEFAULT_SETTINGS, ...(pid ? server.data : {}), ...local }),
    [pid, server.data, local],
  )

  const isWatched = useCallback(
    (slug: string, variable: string) => settings.watched.includes(watchKey(slug, variable)),
    [settings.watched],
  )

  const toggleWatch = useCallback(
    (slug: string, variable: string) => {
      const key = watchKey(slug, variable)
      const watched = settings.watched.includes(key)
        ? settings.watched.filter((k) => k !== key)
        : [...settings.watched, key]
      update({ watched })
    },
    [settings.watched, update],
  )

  // Theme is global chrome, so apply it where CSS can see it.
  useEffect(() => {
    document.documentElement.dataset.theme = settings.theme
  }, [settings.theme])

  const value = useMemo(
    () => ({ settings, update, isWatched, toggleWatch }),
    [settings, update, isWatched, toggleWatch],
  )
  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>
}

export function useSettings(): SettingsContextValue {
  const ctx = useContext(SettingsContext)
  if (!ctx) throw new Error('useSettings must be used inside <SettingsProvider>')
  return ctx
}
