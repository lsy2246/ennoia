import { defineConfig } from "@pandacss/dev";

export default defineConfig({
  preflight: true,
  include: ["./src/**/*.{js,jsx,ts,tsx}"],
  exclude: [],
  outdir: "styled-system",
  theme: {
    extend: {
      tokens: {
        colors: {
          parchment: { value: "#f5f1e8" },
          paper: { value: "#fff8ec" },
          paperSolid: { value: "#ffffff" },
          ink: { value: "#1e1d1a" },
          muted: { value: "#6d675f" },
          copper: { value: "#8c6132" },
          ember: { value: "#b75d24" },
          sky: { value: "#59a8ff" },
          sandLine: { value: "rgba(30, 29, 26, 0.08)" },
          shellGlow: { value: "rgba(84, 60, 28, 0.12)" },
        },
        fonts: {
          body: { value: '"Aptos", "Segoe UI", sans-serif' },
          display: {
            value: '"Iowan Old Style", "Palatino Linotype", "Book Antiqua", serif',
          },
        },
        radii: {
          panel: { value: "24px" },
          pill: { value: "999px" },
        },
        shadows: {
          card: { value: "0 18px 48px rgba(84, 60, 28, 0.10)" },
          button: { value: "0 12px 30px rgba(84, 60, 28, 0.12)" },
        },
      },
    },
  },
  globalCss: {
    "html, body": {
      minHeight: "100%",
    },
    "*, *::before, *::after": {
      boxSizing: "border-box",
    },
    body: {
      margin: 0,
      color: "ink",
      fontFamily: "body",
      background:
        "radial-gradient(circle at top left, rgba(255, 196, 112, 0.55), transparent 28%), radial-gradient(circle at right, rgba(89, 168, 255, 0.22), transparent 30%), #f5f1e8",
      lineHeight: 1.5,
      minHeight: "100vh",
    },
    "#root": {
      minHeight: "100vh",
    },
    "button, input, select": {
      font: "inherit",
    },
  },
});
