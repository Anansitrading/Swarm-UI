import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import MiniSearch from "minisearch";
import { invoke } from "@tauri-apps/api/core";
import type { SessionInfo } from "../types/session";
import { useSessionStore } from "../stores/sessionStore";

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
    const [inputValue, setInputValue] = useState("");
    const [query, setQuery] = useState(""); // Debounced query
    // Tracks sessions whose metadata has been indexed
    const metaIndexedRef = useRef<Set<string>>(new Set());
    // Tracks sessions whose conversation text has been fetched and indexed
    const contentIndexedRef = useRef<Set<string>>(new Set());
    const [isIndexing, setIsIndexing] = useState(false);
    // Trigger re-render after content indexing batches complete
    const [contentIndexVersion, setContentIndexVersion] = useState(0);
    const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    const setStoreQuery = useSessionStore(s => s.setSearchQuery);

    // Debounce: update actual query 200ms after user stops typing
    useEffect(() => {
        if (debounceRef.current) clearTimeout(debounceRef.current);
        debounceRef.current = setTimeout(() => {
            setQuery(inputValue);
        }, 200);
        return () => {
            if (debounceRef.current) clearTimeout(debounceRef.current);
        };
    }, [inputValue]);

    // Sync debounced query to store so SessionDetail can read it
    useEffect(() => {
        setStoreQuery(query);
    }, [query, setStoreQuery]);

    // Create MiniSearch instance once
    if (!miniSearchRef.current) {
        miniSearchRef.current = new MiniSearch<SearchableSession>({
            fields: ["projectName", "projectPath", "branch", "model", "sessionId", "conversationText"],
            storeFields: ["sessionId"],
            searchOptions: {
                boost: { conversationText: 2, projectName: 3, branch: 2, sessionId: 1.5 },
                prefix: true,
                fuzzy: 0.2,
                combineWith: "OR",
            },
        });
    }

    // Index metadata for all sessions (fast - no disk IO)
    useEffect(() => {
        const ms = miniSearchRef.current!;
        const newSessions: SearchableSession[] = [];

        for (const s of sessions) {
            if (!metaIndexedRef.current.has(s.jsonl_path)) {
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
                metaIndexedRef.current.add(s.jsonl_path);
            }
        }

        if (newSessions.length > 0) {
            ms.addAll(newSessions);
        }
    }, [sessions]);

    // Lazily fetch and index conversation text in background batches
    useEffect(() => {
        if (isIndexing) return;
        const ms = miniSearchRef.current!;

        // Find sessions that have metadata indexed but NOT conversation text yet
        const needsContent = sessions
            .filter(s => metaIndexedRef.current.has(s.jsonl_path) && !contentIndexedRef.current.has(s.jsonl_path))
            .map(s => s.jsonl_path);

        if (needsContent.length === 0) return;

        // Process in batches of 50 (Rust uses rayon for parallel file I/O)
        const batch = needsContent.slice(0, 50);
        setIsIndexing(true);

        invoke<[string, string][]>("get_sessions_search_text", { jsonlPaths: batch })
            .then(results => {
                for (const [jsonlPath, text] of results) {
                    // Mark as content-indexed regardless of whether text was empty
                    contentIndexedRef.current.add(jsonlPath);

                    if (text) {
                        // Remove old doc and re-add with conversation text
                        try {
                            ms.discard(jsonlPath);
                        } catch {
                            // May fail if already discarded, that's fine
                        }
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
                }
                // Bump version to trigger search re-evaluation and next batch
                setContentIndexVersion(v => v + 1);
            })
            .catch(e => console.error("Search indexing failed:", e))
            .finally(() => setIsIndexing(false));
    }, [sessions, isIndexing, contentIndexVersion]);

    // Perform search (re-evaluate when content index updates)
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
    }, [query, contentIndexVersion]); // eslint-disable-line react-hooks/exhaustive-deps

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
        query: inputValue,
        setQuery: setInputValue,
        debouncedQuery: query,
        results,
        matchedSessionIds,
        isSearching: query.trim().length > 0,
        isIndexing,
        getHighlightRanges,
    };
}
