export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        dark: {
          900: '#111827',
          800: '#1f2937',
          700: '#374151',
          600: '#4b5563',
        },
        brand: {
          green: '#22c55e',
          red: '#ef4444',
          yellow: '#eab308'
        }
      }
    },
  },
  plugins: [],
}
