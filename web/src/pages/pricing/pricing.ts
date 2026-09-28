/** Whole-percent saving of paying yearly over twelve monthly payments; 0 when there is none. */
export function yearlySavings(monthly: number | undefined, yearly: number | undefined): number {
  if (!monthly || !yearly) return 0
  const pct = Math.round((1 - yearly / (monthly * 12)) * 100)
  return pct > 0 ? pct : 0
}

/** "$9" or "$8.25" — prices are USD (`PlanInfo`); cents only when there are some. */
export function formatPrice(amount: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: Number.isInteger(amount) ? 0 : 2,
    maximumFractionDigits: 2,
  }).format(amount)
}
