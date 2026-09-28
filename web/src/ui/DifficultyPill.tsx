import type { Difficulty } from '@/api/types'

/** Easy / Medium / Hard in green / amber / red, on a 13% tint of the same. */
export function DifficultyPill({ difficulty, short }: { difficulty: Difficulty; short?: boolean }) {
  const label = short && difficulty === 'Medium' ? 'Med' : difficulty
  return <span className={`diff diff-${difficulty.toLowerCase()}`}>{label}</span>
}
