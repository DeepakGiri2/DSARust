// "Chrome on Windows" from a User-Agent string, for the sessions list. Only
// the families a person would recognise; order matters, because every UA
// claims to be several things (Edge says Chrome, Chrome says Safari, iOS says
// Mac OS X, Android says Linux).

const BROWSERS: [RegExp, string][] = [
  [/Edg(A|iOS)?\//, 'Edge'],
  [/OPR\/|Opera/, 'Opera'],
  [/Firefox\/|FxiOS\//, 'Firefox'],
  [/Chrome\/|CriOS\//, 'Chrome'],
  [/Safari\//, 'Safari'],
]

const SYSTEMS: [RegExp, string][] = [
  [/iPhone|iPad|iPod/, 'iOS'],
  [/Android/, 'Android'],
  [/Windows/, 'Windows'],
  [/CrOS/, 'ChromeOS'],
  [/Mac OS X|Macintosh/, 'macOS'],
  [/Linux/, 'Linux'],
]

const first = (ua: string, table: [RegExp, string][]) => table.find(([re]) => re.test(ua))?.[1]

export function describeDevice(ua: string | null): string {
  if (!ua) return 'Unknown device'
  const browser = first(ua, BROWSERS)
  const os = first(ua, SYSTEMS)
  if (browser && os) return `${browser} on ${os}`
  return browser ?? os ?? 'Unknown device'
}
