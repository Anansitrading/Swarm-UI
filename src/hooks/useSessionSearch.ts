import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import MiniSearch from "minisearch";
import { invoke } from "@tauri-apps/api/core";
import type { SessionInfo } from "../types/session";

interface SearchableSession {
    id: string;
    jsonlPath: string;
    projectName: string;
    projectPath: string;
    branch: string;
    model: string;
    sessionId: string;
    conversationText: string;
}

export interface SearchResult {
    sessionId: string;
    score: number;
    matchedFields: string[];
}

export function useSessionSearch(sessions: SessionInfo[]) {
    const miniSearchRef = useRef<MiniSearch<SearchableSession> | null>(null);
    const [query, setQuery] = useState("");
    const [indexedPaths, setIndexedPaths] = useState<Set<string>>(new Set());
    const [isIndexing, setIsIndexing] = useState(false);

    // Create MiniSearch instance once
    if (!miniSearchRef.current) {
        miniSearchRef.current = new MiniSearch<SearchableSession>({
            fields: ["projectName", "projectPath", "branch", "model", "sessionId", "conversationText"],
            storeFields: ["sessionId"],
            searchOptions: {
                boost: { projectName: 3, branch: 2, sessionId: 1.5, conversationText: 1 },
                prefix: true,
                fuzzy: 0.2,
                combineWith: "AND",
            },
        });
    }

    // Index metadata for all sessions (fast - no disk IO)
    useEffect(() => {
        const ms = miniSearchRef.current!;
        const newSessions: SearchableSession[] = [];

        for (const s of sessions) {
            if (!indexedPaths.has(s.jsonl_path)) {
                newSessions.push({
                    id: s.jsonl_path, // MiniSearch needs unique `id`
                    jsonlPath: s.jsonl_path,
                    projectName: s.project_path.split("/").pop() || "",
                    projectPath: s.project_path,
                    branch: s.git_branch || "",
                    model: s.model || "",
                    sessionId: s.id,
                    conversationText: "", // Filled lazily
                });
            }
        }

        if (newSessions.length > 0) {
            ms.addAll(newSessions);
            setIndexedPaths(prev => {
                const next = new Set(prev);
                for (const s of newSessions) next.add(s.jsonlPath);
                return next;
            });
        }
    }, [sessions, indexedPaths]);

    // Lazily fetch and index conversation text in background
    useEffect(() => {
        if (isIndexing) return;
        const ms = miniSearchRef.current!;

        // Find sessions that need conversation text indexed
        const unindexedPaths = sessions
            .filter(s => indexedPaths.has(s.jsonl_path))
            .map(s => s.jsonl_path);

        if (unindexedPaths.length === 0) return;

        // Index in batches of 20
        const batch = unindexedPaths.slice(0, 20);
        setIsIndexing(true);

        invoke<[string, string][]>("get_sessions_search_text", { jsonlPaths: batch })
            .then(results => {
                for (const [jsonlPath, text] of results) {
                    if (text) {
                        // Remove old doc and re-add with conversation text
                        try {
                            const existing = ms.getStoredFields(jsonlPath);
                            if (existing) {
                                ms.discard(jsonlPath);
                                const session = sessions.find(s => s.jsonl_path === jsonlPath);
                                if (session) {
                                    ms.add({
                                        id: jsonlPath,
                                        jsonlPath,
                                        projectName: session.project_path.split("/").pop() || "",
                                        projectPath: session.project_path,
                                        branch: session.git_branch || "",
                                        model: session.model || "",
                                        sessionId: session.id,
                                        conversationText: text,
                                    });
                                }
                            }
                        } catch {
                            // Ignore errors during re-indexing
                        }
                    }
                }
            })
            .catch(e => console.error("Search indexing failed:", e))
            .finally(() => setIsIndexing(false));
    }, [sessions, indexedPaths, isIndexing]);

    // Perform search
    const results = useMemo((): SearchResult[] | null => {
        if (!query.trim()) return null;
        const ms = miniSearchRef.current!;
        try {
            return ms.search(query).map(r => ({
                sessionId: (r as any).sessionId || r.id,
                score: r.score,
                matchedFields: Object.keys(r.match),
            }));
        } catch {
            return null;
        }
    }, [query]);

    // Get matched session IDs set for fast lookup
    const matchedSessionIds = useMemo(() => {
        if (!results) return null;
        return new Set(results.map(r => r.sessionId));
    }, [results]);

    // Highlight function - wraps matched terms in a string with <mark> tags
    const getHighlightRanges = useCallback((text: string): { start: number; end: number }[] => {
        if (!query.trim() || !text) return [];
        const terms = query.trim().toLowerCase().split(/\s+/);
        const ranges: { start: number; end: number }[] = [];
        const lower = text.toLowerCase();

        for (const term of terms) {
            if (!term) continue;
            let pos = 0;
            while (pos < lower.length) {
                const idx = lower.indexOf(term, pos);
                if (idx === -1) break;
                ranges.push({ start: idx, end: idx + term.length });
                pos = idx + 1;
            }
        }

        // Merge overlapping ranges
        ranges.sort((a, b) => a.start - b.start);
        const merged: { start: number; end: number }[] = [];
        for (const r of ranges) {
            const last = merged[merged.length - 1];
            if (last && r.start <= last.end) {
                last.end = Math.max(last.end, r.end);
            } else {
                merged.push({ ...r });
            }
        }
        return merged;
    }, [query]);

    return {
        query,
        setQuery,
        results,
        matchedSessionIds,
        isSearching: query.trim().length > 0,
        isIndexing,
        getHighlightRanges,
    };
}
