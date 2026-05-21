/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'Noto Sans SC', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      colors: {
        accent: 'var(--role-accent)',
        sidebar: 'var(--sidebar-bg)',
        surface: 'var(--bg-surface)',
        elevated: 'var(--bg-elevated)',
        'energy-high': 'var(--energy-high)',
        'energy-mid': 'var(--energy-mid)',
        'energy-low': 'var(--energy-low)',
      },
      borderRadius: {
        button: 'var(--radius-button)',
        card: 'var(--radius-card)',
        dialog: 'var(--radius-dialog)',
        input: 'var(--radius-input)',
      },
      transitionDuration: {
        fast: 'var(--duration-fast)',
        normal: 'var(--duration-normal)',
        color: 'var(--duration-color)',
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
}
