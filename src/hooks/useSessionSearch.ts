import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import type { SessionListItem } from "../types/session";
import { useSessionStore } from "../stores/sessionStore";

/**
 * Lightweight session search — filters against metadata only (zero disk IO).
 *
 * Searchable fields (all present on SessionListItem):
 *   - project_path / project name
 *   - git_branch
 *   - model
 *   - session_id
 *   - summary
 *   - first_prompt
 *
 * Full conversation search is done via invoke("search_sessions") separately.
 * This keeps the session list snappy even with 21K+ sessions.
 */
export function useSessionSearch(sessions: SessionListItem[]) {
    const [inputValue, setInputValue] = useState("");
    const [query, setQuery] = useState(""); // Debounced query
    const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    const setStoreQuery = useSessionStore((s) => s.setSearchQuery);

    // Debounce: update actual query 150ms after user stops typing
    useEffect(() => {
        if (debounceRef.current) clearTimeout(debounceRef.current);
        debounceRef.current = setTimeout(() => {
            setQuery(inputValue);
        }, 150);
        return () => {
            if (debounceRef.current) clearTimeout(debounceRef.current);
        };
    }, [inputValue]);

    // Sync debounced query to store so SessionDetail can read it
    useEffect(() => {
        setStoreQuery(query);
    }, [query, setStoreQuery]);

    // Get matched session IDs — simple case-insensitive substring match
    const matchedSessionIds = useMemo(() => {
        const q = query.trim().toLowerCase();
        if (!q) return null;

        const terms = q.split(/\s+/).filter(Boolean);
        const matched = new Set<string>();

        for (const session of sessions) {
            // Build searchable text from metadata fields
            const searchable = [
                session.project_path,
                session.project_path.split("/").pop() || "",
                session.git_branch || "",
                session.model || "",
                session.session_id,
                session.summary || "",
                session.first_prompt || "",
            ]
                .join(" ")
                .toLowerCase();

            // All terms must match (AND logic)
            const allMatch = terms.every((term) => searchable.includes(term));
            if (allMatch) {
                matched.add(session.session_id);
            }
        }

        return matched;
    }, [sessions, query]);

    // Highlight function — wraps matched terms in a string
    const getHighlightRanges = useCallback(
        (text: string): { start: number; end: number }[] => {
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
        },
        [query],
    );

    return {
        query: inputValue,
        setQuery: setInputValue,
        debouncedQuery: query,
        matchedSessionIds,
        isSearching: query.trim().length > 0,
        getHighlightRanges,
    };
}
