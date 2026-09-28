import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { baseRoutes, mockApi, renderRoutes } from '@/pages/shell/test-utils'
import { HelperModal } from './HelperModal'

afterEach(() => vi.unstubAllGlobals())

const onClose = vi.fn()

function open(category = 'Arrays & Hashing', lang = 'cpp') {
  mockApi(baseRoutes(null))
  return renderRoutes(
    [{ path: '/', element: <HelperModal open onClose={onClose} category={category} lang={lang} /> }],
    '/',
  )
}

describe('📘 helper', () => {
  it('opens on the category’s first topic and walks the list with the arrow keys', async () => {
    const user = userEvent.setup()
    open()
    expect(await screen.findByRole('heading', { level: 3, name: 'Hash Map' })).toBeInTheDocument()
    expect(screen.getByText('for “Arrays & Hashing”')).toBeInTheDocument()
    const first = screen.getByRole('button', { name: 'Hash Map' })
    expect(first).toHaveAttribute('aria-current', 'true')
    first.focus()
    await user.keyboard('{ArrowDown}')
    expect(screen.getByRole('heading', { level: 3, name: 'Array' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Array' })).toHaveFocus()
    // Across the group boundary, into "everything else".
    await user.keyboard('{End}')
    expect(screen.getByRole('heading', { level: 3, name: 'Two Pointers' })).toBeInTheDocument()
  })

  it('shows the syntax for the problem’s language, switchable in place', async () => {
    const user = userEvent.setup()
    open()
    await user.click(await screen.findByRole('button', { name: 'Array' }))
    expect(screen.getByText('vector<int> nums;')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Go' }))
    expect(screen.getByText('nums := []int{1}')).toBeInTheDocument()
  })

  it('compares two languages in the cheat sheet, and swaps them', async () => {
    const user = userEvent.setup()
    open('Arrays & Hashing', 'cpp')
    await user.click(await screen.findByRole('button', { name: '⇄ syntax cheat sheet' }))
    const heads = () => screen.getAllByRole('columnheader').map((h) => h.textContent)
    expect(heads()).toEqual(['what', 'C++', 'Go'])
    await user.click(screen.getByRole('button', { name: '⇄ swap' }))
    expect(heads()).toEqual(['what', 'Go', 'C++'])
    await user.type(screen.getByRole('searchbox', { name: 'Filter the cheat sheet' }), 'func')
    expect(screen.getAllByRole('row')).toHaveLength(2) // header + "function"
  })

  it('closes on Esc', async () => {
    const user = userEvent.setup()
    open()
    await screen.findByRole('heading', { level: 3, name: 'Hash Map' })
    await user.keyboard('{Escape}')
    expect(onClose).toHaveBeenCalled()
  })
})
