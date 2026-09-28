import { API_BASE } from '@/api/client'
import type { Meta } from '@/api/types'
import styles from './auth.module.css'

/**
 * "Continue with …" for each provider the server has configured. These are
 * full-page navigations, not fetches: the provider's consent screen has to be
 * a real page, and the callback sets the session cookie on the way back.
 */
export function OAuthButtons({ oauth, next }: { oauth: Meta['features']['oauth'] | undefined; next: string }) {
  if (!oauth || (!oauth.github && !oauth.google)) return null
  const href = (provider: 'github' | 'google') =>
    `${API_BASE}/auth/oauth/${provider}/start?next=${encodeURIComponent(next)}`
  return (
    <>
      <div className={styles.oauth}>
        {oauth.github && (
          <a className="btn btn-ghost btn-block" href={href('github')}>
            Continue with GitHub
          </a>
        )}
        {oauth.google && (
          <a className="btn btn-ghost btn-block" href={href('google')}>
            Continue with Google
          </a>
        )}
      </div>
      <div className={styles.or} aria-hidden>
        or
      </div>
    </>
  )
}

/** The codes `backend/crates/api/src/routes/oauth.rs` redirects with. */
const OAUTH_ERRORS: Record<string, string> = {
  oauth_denied: 'Sign-in was cancelled at the provider — nothing was changed.',
  oauth_expired: 'That sign-in took too long or was opened in another tab. Please try again.',
  oauth_email_in_use:
    'An account with that email already exists, and the provider hasn’t verified the address. Sign in with your password instead.',
}

/** The callback lands on `/login?error=<code>` when it could not finish. */
export function oauthErrorMessage(code: string): string {
  return (
    OAUTH_ERRORS[code] ??
    'Signing in with that provider didn’t work. Try again, or use your email and password.'
  )
}
