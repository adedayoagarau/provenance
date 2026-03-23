import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{js,ts,jsx,tsx,mdx}"],
  theme: {
    extend: {
      colors: {
        bg: {
          primary: "#0F1117",
          secondary: "#161821",
          tertiary: "#1C1E2A",
          elevated: "#232636",
        },
        border: {
          DEFAULT: "#2A2D3E",
          focus: "#4A4D5E",
        },
        text: {
          primary: "#E8E9ED",
          secondary: "#9BA1B0",
          tertiary: "#6B7280",
        },
        status: {
          blue: "#3B82F6",
          "blue-dim": "#1E3A5F",
          amber: "#F59E0B",
          "amber-dim": "#5C3D0E",
          red: "#EF4444",
          "red-dim": "#5C1A1A",
          gray: "#6B7280",
          "gray-dim": "#2A2D3E",
        },
        accent: "#3B82F6",
      },
      fontFamily: {
        mono: ['"JetBrains Mono"', '"IBM Plex Mono"', "Consolas", "monospace"],
        sans: ['"General Sans"', '"Inter"', "system-ui", "sans-serif"],
      },
    },
  },
  plugins: [],
};
export default config;
