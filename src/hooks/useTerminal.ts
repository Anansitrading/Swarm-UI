import { useEffect, useRef, useCallback } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useTerminalStore } from "../stores/terminalStore";
import "@xterm/xterm/css/xterm.css";

interface UseTerminalOptions {
    ptyId: string;
    containerRef: React.RefObject<HTMLDivElement | null>;
}

export function useTerminal({ ptyId, containerRef }: UseTerminalOptions) {
    const terminalRef = useRef<Terminal | null>(null);
    const fitAddonRef = useRef<FitAddon | null>(null);
    const unlistenRef = useRef<UnlistenFn | null>(null);

    const { writeToTerminal, resizeTerminal } = useTerminalStore();

    const initTerminal = useCallback(async () => {
        if (!containerRef.current || terminalRef.current) return;

        const term = new Terminal({
            cursorBlink: true,
            fontSize: 13,
            fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
            theme: {
                background: "#0a0a0f",
                foreground: "#e2e8f0",
                cursor: "#7c3aed",
                cursorAccent: "#0a0a0f",
                selectionBackground: "#7c3aed40",
                black: "#1e1e2e",
                red: "#ef4444",
                green: "#22c55e",
                yellow: "#f59e0b",
                blue: "#3b82f6",
                magenta: "#a855f7",
                cyan: "#06b6d4",
                white: "#e2e8f0",
                brightBlack: "#64748b",
                brightRed: "#f87171",
                brightGreen: "#4ade80",
                brightYellow: "#fbbf24",
                brightBlue: "#60a5fa",
                brightMagenta: "#c084fc",
                brightCyan: "#22d3ee",
                brightWhite: "#f8fafc",
            },
            allowProposedApi: true,
        });

        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);

        term.open(containerRef.current);

        // Try WebGL renderer for performance
        try {
            const webglAddon = new WebglAddon();
            term.loadAddon(webglAddon);
        } catch {
            console.warn("WebGL addon failed, using canvas renderer");
        }

        fitAddon.fit();

        // Handle user input -> PTY
        // Convert the JS string to UTF-8 bytes, then base64 encode
        term.onData((data) => {
            const encoder = new TextEncoder();
            const bytes = encoder.encode(data);
            let binary = "";
            for (let i = 0; i < bytes.length; i++) {
                binary += String.fromCharCode(bytes[i]);
            }
            writeToTerminal(ptyId, btoa(binary));
        });

        // Handle resize -> PTY
        term.onResize(({ cols, rows }) => {
            resizeTerminal(ptyId, cols, rows);
        });

        // Listen for PTY output -> terminal
        // Decode base64 to Uint8Array for proper binary/UTF-8 handling
        const unlisten = await listen<string>(`pty:data:${ptyId}`, (event) => {
            try {
                const binaryStr = atob(event.payload);
                const bytes = new Uint8Array(binaryStr.length);
                for (let i = 0; i < binaryStr.length; i++) {
                    bytes[i] = binaryStr.charCodeAt(i);
                }
                term.write(bytes);
            } catch {
                // Handle non-base64 data as raw string fallback
                term.write(event.payload);
            }
        });

        // Listen for PTY exit
        const unlistenExit = await listen(`pty:exit:${ptyId}`, () => {
            term.write("\r\n\x1b[90m[Process exited]\x1b[0m\r\n");
        });

        terminalRef.current = term;
        fitAddonRef.current = fitAddon;
        unlistenRef.current = () => {
            unlisten();
            unlistenExit();
        };

        // Initial resize notification
        resizeTerminal(ptyId, term.cols, term.rows);
    }, [ptyId, containerRef, writeToTerminal, resizeTerminal]);

    // Handle container resize
    const handleResize = useCallback(() => {
        if (fitAddonRef.current && terminalRef.current) {
            fitAddonRef.current.fit();
        }
    }, []);

    useEffect(() => {
        initTerminal();

        const observer = new ResizeObserver(handleResize);
        if (containerRef.current) {
            observer.observe(containerRef.current);
        }

        return () => {
            observer.disconnect();
            unlistenRef.current?.();
            terminalRef.current?.dispose();
            terminalRef.current = null;
        };
    }, [initTerminal, handleResize, containerRef]);

    return {
        terminal: terminalRef,
        fit: handleResize,
    };
}
