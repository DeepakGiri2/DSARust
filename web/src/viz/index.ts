// Public surface of the visualization layer — a port of `crates/dsa-viz`.
//
// CONTRACT: the problem page imports exactly these names. The implementation
// behind them may change freely; these signatures may not without updating
// every caller.

export { VizCanvas, type VizCanvasProps } from './VizCanvas'
export { VIEW_SPACING, viewHeight } from './render'
export { describeView } from './a11y'
export { useVizTheme } from './hooks'
export { DARK_THEME, LIGHT_THEME, themeFromDocument, type VizTheme } from './theme'
