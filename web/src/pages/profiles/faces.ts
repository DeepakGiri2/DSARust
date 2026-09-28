import type { CSSProperties } from 'react'
import { profileColor } from '@/pages/shell/profileColor'

/** `--c` drives a face's tile, ring and swatch colours in profiles.module.css. */
export const faceVars = (color: string): CSSProperties => ({ ['--c' as string]: profileColor(color) })
