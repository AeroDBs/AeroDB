export default {
    content: ['./index.html', './src/**/*.{vue,js,ts,jsx,tsx}'],
    theme: {
        extend: {
            fontFamily: {
                sans: ['Lato', 'sans-serif'],
            },
            colors: {
                border: 'hsl(var(--color-border))',
                input: 'hsl(var(--color-input))',
                ring: 'hsl(var(--color-ring))',
                background: 'hsl(var(--color-background))',
                foreground: 'hsl(var(--color-foreground))',
                primary: {
                    DEFAULT: 'hsl(var(--color-primary))',
                    foreground: 'hsl(var(--color-primary-foreground))',
                },
                secondary: {
                    DEFAULT: 'hsl(var(--color-secondary))',
                    foreground: 'hsl(var(--color-secondary-foreground))',
                },
                destructive: {
                    DEFAULT: 'hsl(var(--color-destructive))',
                    foreground: 'hsl(var(--color-destructive-foreground))',
                },
                muted: {
                    DEFAULT: 'hsl(var(--color-muted))',
                    foreground: 'hsl(var(--color-muted-foreground))',
                },
                accent: {
                    DEFAULT: 'hsl(var(--color-accent))',
                    foreground: 'hsl(var(--color-accent-foreground))',
                },
                card: {
                    DEFAULT: 'hsl(var(--color-card))',
                    foreground: 'hsl(var(--color-card-foreground))',
                },
            },
        },
    },
}
