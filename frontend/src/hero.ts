import { heroui } from "@heroui/theme";

export default heroui({
  defaultTheme: "dark",
  defaultExtendTheme: "dark",
  layout: {
    radius: {
      small: "4px",
      medium: "8px",
      large: "12px",
    },
  },
  themes: {
    dark: {
      colors: {
        background: "#020617", // slate-950
        foreground: "#f8fafc",
        default: {
          50: "#f8fafc",
          100: "#1e293b", // slate-800
          200: "#334155", // slate-700
          300: "#475569", // slate-600
          400: "#64748b", // slate-500
          500: "#94a3b8", // slate-400
          600: "#cbd5e1", // slate-300
          700: "#e2e8f0", // slate-200
          800: "#f1f5f9", // slate-100
          900: "#f8fafc", // slate-50
          DEFAULT: "#1e293b",
          foreground: "#f8fafc",
        },
        content1: "#0f172a", // slate-900
        content2: "#1e293b", // slate-800
        content3: "#334155", // slate-700
        content4: "#475569", // slate-600
        primary: {
          50: "#eff6ff",
          100: "#dbeafe",
          200: "#bfdbfe",
          300: "#93c5fd",
          400: "#60a5fa",
          500: "#3b82f6",
          600: "#2563eb",
          700: "#1d4ed8",
          800: "#1e40af",
          900: "#1e3a8a",
          DEFAULT: "#3b82f6",
          foreground: "#ffffff",
        },
        focus: "#3b82f6",
      },
    },
  },
});
