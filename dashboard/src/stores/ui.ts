import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useUIStore = defineStore('ui', () => {
    // State
    const sidebarOpen = ref(true)
    const theme = ref<'light' | 'dark'>('dark')

    // Load theme preference from localStorage
    const savedTheme = localStorage.getItem('theme') as 'light' | 'dark' | null
    if (savedTheme) {
        theme.value = savedTheme
    }

    // Apply theme to document
    const applyTheme = () => {
        if (theme.value === 'dark') {
            document.documentElement.classList.add('dark')
        } else {
            document.documentElement.classList.remove('dark')
        }
    }

    // Initialize theme
    applyTheme()

    // Actions
    const toggleSidebar = () => {
        sidebarOpen.value = !sidebarOpen.value
    }

    const setTheme = (newTheme: 'light' | 'dark') => {
        theme.value = newTheme
        localStorage.setItem('theme', newTheme)
        applyTheme()
    }

    const toggleTheme = () => {
        setTheme(theme.value === 'dark' ? 'light' : 'dark')
    }

    return {
        // State
        sidebarOpen,
        theme,

        // Actions
        toggleSidebar,
        setTheme,
        toggleTheme,
    }
})
