// The route table. Every page is code-split: its module exports `Component`
// (React Router's lazy-route convention), so the catalogue never downloads the
// editor and the problem page never downloads the account screens.
//
// Ownership (for parallel work): each page module lives in its own directory
// under src/pages or src/features, and only this file wires them together.

import { createBrowserRouter } from 'react-router'
import { RootLayout } from './RootLayout'
import { RouteError } from './RouteError'

export const router = createBrowserRouter([
  {
    element: <RootLayout />,
    errorElement: <RouteError />,
    children: [
      {
        // Pages with the site chrome (top bar, account menu).
        lazy: () => import('@/pages/shell/AppShell'),
        children: [
          { index: true, lazy: () => import('@/pages/catalog/CatalogPage') },
          { path: 'login', lazy: () => import('@/pages/auth/LoginPage') },
          { path: 'signup', lazy: () => import('@/pages/auth/SignupPage') },
          { path: 'forgot-password', lazy: () => import('@/pages/auth/ForgotPasswordPage') },
          { path: 'reset-password', lazy: () => import('@/pages/auth/ResetPasswordPage') },
          { path: 'verify-email', lazy: () => import('@/pages/auth/VerifyEmailPage') },
          { path: 'profiles', lazy: () => import('@/pages/profiles/ProfilesPage') },
          { path: 'dashboard', lazy: () => import('@/pages/dashboard/DashboardPage') },
          { path: 'account', lazy: () => import('@/pages/account/AccountPage') },
          { path: 'pricing', lazy: () => import('@/pages/pricing/PricingPage') },
          { path: 'admin', lazy: () => import('@/pages/admin/AdminPage') },
          { path: '*', lazy: () => import('@/pages/shell/NotFoundPage') },
        ],
      },
      // The problem workspace is full-screen, like the desktop: its own header,
      // no site chrome. `tab` is `practice` (default) or `visualize`.
      { path: 'problems/:slug', lazy: () => import('@/pages/problem/ProblemPage') },
      { path: 'problems/:slug/:tab', lazy: () => import('@/pages/problem/ProblemPage') },
      // Development only: every view kind rendered from real engine traces.
      ...(import.meta.env.DEV
        ? [{ path: 'dev/viz', lazy: () => import('@/viz/gallery/VizGalleryPage') }]
        : []),
    ],
  },
])
