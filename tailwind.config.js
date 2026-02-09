/** @type {import('tailwindcss').Config} */
export default {
    content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
    darkMode: "class",
    theme: {
        extend: {
            colors: {
                swarm: {
                    bg: "#0a0a0f",
                    surface: "#12121a",
                    border: "#1e1e2e",
                    accent: "#dc2626",
                    "accent-dim": "#991b1b",
                    success: "#22c55e",
                    warning: "#f59e0b",
                    danger: "#ef4444",
                    info: "#3b82f6",
                    text: "#e2e8f0",
                    "text-dim": "#94a3b8",
                },
            },
            fontFamily: {
                mono: ["JetBrains Mono", "Fira Code", "monospace"],
            },
        },
    },
    plugins: [],
};
