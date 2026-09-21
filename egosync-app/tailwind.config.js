/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      // Story 17.1（人工裁决 B）：Google Fonts 外链改 @fontsource-variable
      // npm 包自托管（import 见 src/index.css）。@fontsource 可变字体
      // 注册的 family 名带 " Variable" 后缀（单文件覆盖全字重 100-900）；
      // 字体栈本身不变（Inter / Noto Sans SC / JetBrains Mono）。
      fontFamily: {
        sans: ['Inter Variable', 'Noto Sans SC Variable', 'sans-serif'],
        mono: ['JetBrains Mono Variable', 'monospace'],
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
  plugins: [require("tailwindcss-animate"), require("@tailwindcss/typography")],
}
