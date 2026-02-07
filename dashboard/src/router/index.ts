import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useSetupStore } from '@/stores/setup'

const router = createRouter({
    history: createWebHistory(),
    routes: [
        // ==================
        // Setup Routes (First-Run Wizard)
        // ==================
        {
            path: '/setup',
            component: () => import('@/setup/SetupLayout.vue'),
            meta: { requiresSetup: true },
            children: [
                {
                    path: '',
                    redirect: '/setup/welcome',
                },
                {
                    path: 'welcome',
                    name: 'SetupWelcome',
                    component: () => import('@/setup/steps/WelcomeStep.vue'),
                },
                {
                    path: 'storage',
                    name: 'SetupStorage',
                    component: () => import('@/setup/steps/StorageStep.vue'),
                },
                {
                    path: 'auth',
                    name: 'SetupAuth',
                    component: () => import('@/setup/steps/AuthConfigStep.vue'),
                },
                {
                    path: 'admin',
                    name: 'SetupAdmin',
                    component: () => import('@/setup/steps/AdminUserStep.vue'),
                },
                {
                    path: 'review',
                    name: 'SetupReview',
                    component: () => import('@/setup/steps/ReviewStep.vue'),
                },
                {
                    path: 'complete',
                    name: 'SetupComplete',
                    component: () => import('@/setup/steps/CompleteStep.vue'),
                },
            ],
        },

        // ==================
        // Public Routes
        // ==================
        {
            path: '/login',
            name: 'Login',
            component: () => import('@/pages/Login.vue'),
            meta: { requiresAuth: false },
        },
        {
            path: '/forgot-password',
            name: 'ForgotPassword',
            component: () => import('@/pages/ForgotPassword.vue'),
            meta: { requiresAuth: false },
        },
        {
            path: '/reset-password',
            name: 'ResetPassword',
            component: () => import('@/pages/ResetPassword.vue'),
            meta: { requiresAuth: false },
        },

        // ==================
        // Protected Routes (Require Auth + Setup Complete)
        // ==================
        {
            path: '/',
            name: 'Home',
            component: () => import('@/pages/Home.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/database',
            name: 'Database',
            component: () => import('@/pages/database/DatabasePage.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/database/table/:name',
            name: 'TableBrowser',
            component: () => import('@/pages/database/TableBrowser.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/database/sql',
            name: 'SQLConsole',
            component: () => import('@/pages/database/SQLConsole.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/auth',
            name: 'Auth',
            component: () => import('@/pages/auth/AuthPage.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/storage',
            name: 'Storage',
            component: () => import('@/pages/storage/StoragePage.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/storage/bucket/:name',
            name: 'FileBrowser',
            component: () => import('@/pages/storage/FileBrowser.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/realtime',
            name: 'Realtime',
            component: () => import('@/pages/realtime/RealtimePage.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/functions',
            name: 'Functions',
            component: () => import('@/pages/functions/FunctionsPage.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/functions/:id/edit',
            name: 'FunctionEditor',
            component: () => import('@/pages/functions/FunctionEditor.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/functions/:id/logs',
            name: 'FunctionLogs',
            component: () => import('@/pages/functions/FunctionLogs.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/backup',
            name: 'Backup',
            component: () => import('@/pages/backup/BackupPage.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/restore',
            name: 'Restore',
            component: () => import('@/pages/backup/RestorePage.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/snapshots',
            name: 'Snapshots',
            component: () => import('@/pages/snapshot/SnapshotPage.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/cluster',
            name: 'Cluster',
            component: () => import('@/pages/cluster/ClusterPage.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/logs',
            name: 'Logs',
            component: () => import('@/pages/observability/LogsPage.vue'),
            meta: { requiresAuth: true },
        },
        {
            path: '/metrics',
            name: 'Metrics',
            component: () => import('@/pages/observability/MetricsPage.vue'),
            meta: { requiresAuth: true },
        },
    ],
})

// ==================
// Navigation Guards
// ==================
router.beforeEach(async (to, _from, next) => {
    const setupStore = useSetupStore()
    const authStore = useAuthStore()

    // 1. Check setup status on first navigation
    if (!setupStore.statusChecked) {
        try {
            await setupStore.fetchStatus()
        } catch (err) {
            // If we can't check status, assume uninitialized
            console.error('Failed to check setup status:', err)
        }
    }

    // 2. If not ready, force setup wizard (except for setup routes)
    if (setupStore.status !== 'Ready' && !to.path.startsWith('/setup')) {
        return next('/setup')
    }

    // 3. If ready, block access to setup routes
    if (setupStore.status === 'Ready' && to.path.startsWith('/setup')) {
        return next('/')
    }

    // 4. If setup is complete, check authentication
    if (to.meta.requiresAuth && !authStore.isAuthenticated) {
        return next('/login')
    }

    // 5. Redirect authenticated users away from login
    if (to.path === '/login' && authStore.isAuthenticated) {
        return next('/')
    }

    // All checks passed
    next()
})

export default router
