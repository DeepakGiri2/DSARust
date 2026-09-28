// Who is signed in, and whose progress is on screen.
//
// The account is the login; the *profile* is the desktop's "Who's
// practising?" — an account owns a handful, each with its own progress,
// favourites, playlists and settings. The active profile is remembered per
// account in localStorage, so a single-profile account never sees a picker and
// a shared one is asked once per device.

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { api, isApiError, onUnauthorized, setCsrfToken } from '@/api/client'
import { qk, useProfiles } from '@/api/hooks'
import type {
  Entitlements,
  LoginRequest,
  Profile,
  SessionInfo,
  SignupRequest,
  User,
  Uuid,
} from '@/api/types'

export type SessionStatus = 'loading' | 'guest' | 'authenticated'

export interface SessionContextValue {
  status: SessionStatus
  user: User | null
  entitlements: Entitlements | null
  profiles: Profile[]
  /** Null for guests, and for a multi-profile account that has not picked yet. */
  activeProfile: Profile | null
  /** True when the account has profiles but none is chosen on this device. */
  needsProfilePick: boolean
  setActiveProfile: (id: Uuid | null) => void
  login: (req: LoginRequest) => Promise<SessionInfo>
  signup: (req: SignupRequest) => Promise<SessionInfo>
  logout: () => Promise<void>
  /** Re-read the session (after verifying email, upgrading, editing profiles…). */
  refresh: () => Promise<void>
}

const SessionContext = createContext<SessionContextValue | null>(null)

const profileKey = (userId: Uuid) => `dsa.activeProfile.${userId}`

function readStoredProfile(userId: Uuid): Uuid | null {
  try {
    return localStorage.getItem(profileKey(userId))
  } catch {
    return null
  }
}

function storeProfile(userId: Uuid, id: Uuid | null) {
  try {
    if (id) localStorage.setItem(profileKey(userId), id)
    else localStorage.removeItem(profileKey(userId))
  } catch {
    // Private mode or blocked storage: the choice just lasts for this tab.
  }
}

export function SessionProvider({ children }: { children: ReactNode }) {
  const qc = useQueryClient()
  const [status, setStatus] = useState<SessionStatus>('loading')
  const [session, setSession] = useState<SessionInfo | null>(null)
  const [activeId, setActiveId] = useState<Uuid | null>(null)

  const adopt = useCallback(
    (info: SessionInfo | null) => {
      setSession(info)
      setCsrfToken(info?.csrf_token ?? null)
      if (!info) {
        setStatus('guest')
        setActiveId(null)
        return
      }
      setStatus('authenticated')
      qc.setQueryData(qk.profiles, info.profiles)
      const stored = readStoredProfile(info.user.id)
      const valid = info.profiles.find((p) => p.id === stored)
      if (valid) setActiveId(valid.id)
      else if (info.profiles.length === 1) setActiveId(info.profiles[0].id)
      else setActiveId(null)
    },
    [qc],
  )

  const refresh = useCallback(async () => {
    try {
      const info = await api.get<SessionInfo>('/auth/session', { quiet401: true })
      adopt(info)
    } catch (e) {
      if (isApiError(e) && e.status === 401) adopt(null)
      else {
        // Network trouble on boot: behave as a guest rather than spin forever.
        adopt(null)
      }
    }
  }, [adopt])

  useEffect(() => {
    void refresh()
  }, [refresh])

  // Any 401 from any request means the session is gone (expired, revoked on
  // another device). Drop to guest and clear everything profile-scoped.
  useEffect(
    () =>
      onUnauthorized(() => {
        adopt(null)
        qc.removeQueries({ queryKey: ['profiles'] })
      }),
    [adopt, qc],
  )

  const login = useCallback(
    async (req: LoginRequest) => {
      const info = await api.post<SessionInfo>('/auth/login', req)
      adopt(info)
      return info
    },
    [adopt],
  )

  const signup = useCallback(
    async (req: SignupRequest) => {
      const info = await api.post<SessionInfo>('/auth/signup', {
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        ...req,
      })
      adopt(info)
      return info
    },
    [adopt],
  )

  const logout = useCallback(async () => {
    try {
      await api.post<void>('/auth/logout')
    } finally {
      adopt(null)
      qc.removeQueries({ queryKey: ['profiles'] })
      qc.removeQueries({ queryKey: qk.me })
    }
  }, [adopt, qc])

  const setActiveProfile = useCallback(
    (id: Uuid | null) => {
      setActiveId(id)
      if (session) storeProfile(session.user.id, id)
    },
    [session],
  )

  // Profiles can change underneath (created, renamed, deleted on another
  // device); subscribe to the query so every screen sees the current list.
  const profilesQuery = useProfiles(status === 'authenticated')
  const profiles = useMemo<Profile[]>(
    () => (status === 'authenticated' ? (profilesQuery.data ?? session?.profiles ?? []) : []),
    [status, profilesQuery.data, session],
  )

  const value = useMemo<SessionContextValue>(() => {
    const activeProfile = profiles.find((p) => p.id === activeId) ?? null
    return {
      status,
      user: session?.user ?? null,
      entitlements: session?.entitlements ?? null,
      profiles,
      activeProfile,
      needsProfilePick: status === 'authenticated' && !activeProfile && profiles.length > 0,
      setActiveProfile,
      login,
      signup,
      logout,
      refresh,
    }
  }, [status, session, profiles, activeId, setActiveProfile, login, signup, logout, refresh])

  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>
}

export function useSession(): SessionContextValue {
  const ctx = useContext(SessionContext)
  if (!ctx) throw new Error('useSession must be used inside <SessionProvider>')
  return ctx
}

/** The active profile id, or null. The argument every profile-scoped hook takes. */
export function useActiveProfileId(): Uuid | null {
  return useSession().activeProfile?.id ?? null
}
