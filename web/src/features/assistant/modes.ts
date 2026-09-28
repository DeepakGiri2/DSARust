// The three assistant modes — a port of `Mode` in `crates/dsa-ai/src/lib.rs`.
//
// * Interview nudges with questions and never writes code or names the
//   technique; code blocks it does emit are rendered locked.
// * Guide is a mentor that answers exactly what was asked, and responds to a
//   vague question with clickable `OPTION:` choices.
// * Fix reviews the current solution and returns issues plus a corrected
//   function, shown as a reviewable diff against what the user wrote.
//
// The prompts (and each mode's temperature) live on the server now; what the
// client needs is how each mode presents itself.

import type { AiMode } from '@/api/types'

export interface ModeInfo {
  /** The segmented-control label, as the desktop draws it. */
  tab: string
  label: string
  /** Who a reply is attributed to in the transcript. */
  speaker: string
  placeholder: string
  emptyHint: string
}

export const MODES: Readonly<Record<AiMode, ModeInfo>> = {
  interview: {
    tab: '🎤 Interview',
    label: 'Interview',
    speaker: 'interviewer',
    placeholder: 'ask the interviewer… (Enter to send)',
    emptyHint:
      'Practice like a real interview — the AI nudges you with questions and tiny hints but never reveals code or names the technique. Try "am I on the right track?".',
  },
  guide: {
    tab: '🗺 Guide',
    label: 'Guide',
    speaker: 'mentor',
    placeholder: 'ask the mentor… (Enter to send)',
    emptyHint:
      'A teaching mentor — ask about syntax, patterns, best practices or complexity and it answers exactly what you asked. Vague question? It asks back with clickable choices.',
  },
  fix: {
    tab: '🔧 Fix',
    label: 'Fix',
    speaker: 'mentor',
    // Fix is one-shot, as on the desktop: the code and the last run go with
    // every request, so anything typed is only a pointer for the reviewer.
    placeholder: 'anything to focus on? optional (Enter to analyze)',
    // The desktop opens the proposal in its editor column; here it opens in
    // this panel and reaches the editor only through "apply".
    emptyHint:
      "Points out what's wrong in your solution, then shows the proposed fix as a diff against your code — keep the changes you want and apply them. Run your code or tests first so the AI sees the errors too.",
  },
}

export const MODE_ORDER: readonly AiMode[] = ['interview', 'guide', 'fix']

/** The modes that hold a conversation; Fix holds one analysis at a time. */
export type ChatMode = Exclude<AiMode, 'fix'>

/**
 * How many turns of history are sent back to the model. The code snapshot
 * travels separately, and only with the turn that attached it, so the window
 * stays small — which on a hosted model is also the user's daily quota.
 */
export const HISTORY_TURNS = 12
