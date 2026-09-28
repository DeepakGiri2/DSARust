import { safeNext } from '@/app/guards'

// The pages that send a signed-in visitor straight on, so as a target they
// would bounce forever.
const AUTH_PATHS = /^\/(login|signup)(\/|\?|$)/

/**
 * Where to go once signed in: the `?next=` target, through "Who's
 * practising?" when the account has several profiles and none is chosen here.
 */
export function afterAuth(rawNext: string | null, needsProfilePick: boolean): string {
  const next = safeNext(rawNext)
  const target = AUTH_PATHS.test(next) ? '/' : next
  return needsProfilePick ? `/profiles?next=${encodeURIComponent(target)}` : target
}
