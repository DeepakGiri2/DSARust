import { NotFoundView } from './NotFoundView'
import { usePageTitle } from './usePageTitle'

export function Component() {
  usePageTitle('Not found')
  return <NotFoundView />
}
