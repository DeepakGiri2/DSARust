import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from 'react-router'
import { isApiError } from '@/api/client'
import { SessionProvider } from '@/state/session'
import { SettingsProvider } from '@/state/settings'
import { ToastProvider } from '@/ui'
import { router } from './router'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // A 4xx will not fix itself on retry; a flaky network might.
      retry: (count, err) => !(isApiError(err) && err.status >= 400 && err.status < 500) && count < 2,
      refetchOnWindowFocus: false,
      staleTime: 30_000,
    },
  },
})

export function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ToastProvider>
        <SessionProvider>
          <SettingsProvider>
            <RouterProvider router={router} />
          </SettingsProvider>
        </SessionProvider>
      </ToastProvider>
    </QueryClientProvider>
  )
}
