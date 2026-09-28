// Pricing: the plans the server offers (`Meta.plans`), a monthly/yearly
// switch, and checkout. A guest signs up first and comes back here with the
// interval they chose. With billing switched off the page still explains what
// each plan includes, but sells nothing.

import { useState } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router'
import clsx from 'clsx'
import { isApiError } from '@/api/client'
import { useCheckout, useMeta } from '@/api/hooks'
import type { CheckoutRequest, Meta, PlanInfo } from '@/api/types'
import { describeError } from '@/pages/auth/errors'
import { usePageTitle } from '@/pages/shell/usePageTitle'
import { useSession } from '@/state/session'
import { ErrorState, PageSpinner, Seg, useToast } from '@/ui'
import { formatPrice, yearlySavings } from './pricing'
import styles from './pricing.module.css'

type Interval = CheckoutRequest['interval']

export function Component() {
  usePageTitle('Pricing')
  const meta = useMeta()
  if (meta.isPending) return <PageSpinner />
  if (meta.isError) {
    return (
      <div className={styles.page}>
        <ErrorState error={meta.error} onRetry={() => void meta.refetch()} />
      </div>
    )
  }
  return <Pricing meta={meta.data} />
}

function Pricing({ meta }: { meta: Meta }) {
  const [params] = useSearchParams()
  const billing = meta.features.billing
  const pro = meta.plans.find((p) => p.id === 'pro')
  const savings = yearlySavings(pro?.price_monthly, pro?.price_yearly)
  const asked = params.get('interval')
  const [interval, setBillingInterval] = useState<Interval>(
    asked === 'month' || asked === 'year' ? asked : savings > 0 ? 'year' : 'month',
  )

  return (
    <div className={styles.page}>
      <header className={styles.hero}>
        <h1 className={styles.title}>
          Simple <span className="grad-text">pricing</span>
        </h1>
        <p className={styles.lede}>
          The debugger, the practice editor and your progress are free. Pro unlocks the premium problems and
          raises the limits.
        </p>
        {billing ? (
          pro?.price_monthly !== undefined &&
          pro.price_yearly !== undefined && (
            <Seg
              aria-label="Billing interval"
              value={interval}
              onChange={setBillingInterval}
              options={[
                { value: 'month', label: 'monthly' },
                { value: 'year', label: savings > 0 ? `yearly · save ${savings}%` : 'yearly' },
              ]}
            />
          )
        ) : (
          <p className={styles.notice}>Billing isn’t switched on here, so there is nothing to buy — this is what each plan includes.</p>
        )}
      </header>

      <div className={styles.plans}>
        {meta.plans.map((plan) => (
          <PlanCard key={plan.id} plan={plan} interval={interval} billing={billing} />
        ))}
      </div>

      <section className={styles.faq} aria-labelledby="faq-title">
        <h2 id="faq-title" className={styles.faqTitle}>
          Questions
        </h2>
        <details>
          <summary>What stays free?</summary>
          <p>
            Every free problem with its step-through debugger and animation, the practice editor, progress,
            favourites, playlists and profiles. Premium problems show their statement to everyone; the code and
            the debugger need Pro.
          </p>
        </details>
        <details>
          <summary>Can I switch between monthly and yearly, or cancel?</summary>
          <p>Yes — Manage billing in your account opens the billing portal, where you can change or cancel the plan.</p>
        </details>
        <details>
          <summary>What happens to my progress if I cancel?</summary>
          <p>Nothing is deleted. Your progress, favourites and playlists stay; premium problems lock again.</p>
        </details>
        <details>
          <summary>Who handles the payment?</summary>
          <p>Checkout and billing run on Stripe. Card details go to them and never touch our servers.</p>
        </details>
      </section>
    </div>
  )
}

function PlanCard({ plan, interval, billing }: { plan: PlanInfo; interval: Interval; billing: boolean }) {
  const { user } = useSession()
  const navigate = useNavigate()
  const checkout = useCheckout()
  const toast = useToast()
  const isPro = plan.id === 'pro'
  const current = user?.plan === plan.id
  const price = interval === 'year' ? plan.price_yearly : plan.price_monthly

  const upgrade = () => {
    if (!user) {
      // Sign up first, then land back here with the same choice made.
      navigate(`/signup?next=${encodeURIComponent(`/pricing?interval=${interval}`)}`)
      return
    }
    checkout.mutate(
      { interval },
      {
        onError: (err) =>
          toast.error(
            isApiError(err, 'email_unverified')
              ? 'Verify your email first — the link is in your inbox.'
              : (describeError(err).message ?? 'Could not start checkout.'),
          ),
      },
    )
  }

  return (
    <article className={clsx('card', styles.plan, isPro && styles.featured)} aria-labelledby={`plan-${plan.id}`}>
      <div className={styles.planHead}>
        <h2 id={`plan-${plan.id}`} className={styles.planName}>
          {plan.name}
        </h2>
        {current && <span className="chip chip-green">your plan</span>}
      </div>

      {billing && (
        <p className={styles.price}>
          {isPro && price !== undefined ? (
            <>
              <span className={styles.amount}>{formatPrice(price)}</span>
              <span className={styles.per}>/ {interval === 'year' ? 'year' : 'month'}</span>
              {interval === 'year' && <span className={styles.equiv}>≈ {formatPrice(price / 12)} a month</span>}
            </>
          ) : (
            <>
              <span className={styles.amount}>{formatPrice(price ?? 0)}</span>
              <span className={styles.per}>forever</span>
            </>
          )}
        </p>
      )}

      <ul className={styles.features}>
        {plan.features.map((f) => (
          <li key={f}>
            <span className={styles.tick} aria-hidden>
              ✓
            </span>
            {f}
          </li>
        ))}
      </ul>

      {billing && (
        <div className={styles.cta}>
          {current ? (
            isPro ? (
              <Link className="btn btn-ghost btn-block" to="/account#billing">
                Manage billing
              </Link>
            ) : (
              <button type="button" className="btn btn-ghost btn-block" disabled>
                Current plan
              </button>
            )
          ) : isPro ? (
            <button type="button" className="btn btn-primary btn-block" onClick={upgrade} disabled={checkout.isPending}>
              {checkout.isPending ? 'Opening checkout…' : 'Upgrade to Pro'}
            </button>
          ) : user ? (
            <span className={styles.included}>Everything here is included in Pro.</span>
          ) : (
            <Link className="btn btn-ghost btn-block" to="/signup">
              Get started free
            </Link>
          )}
        </div>
      )}
    </article>
  )
}
