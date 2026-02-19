import { useEffect } from "react";
import { AppShell } from "./components/layout/AppShell";
import { ConfirmationModal } from "./components/sprite/ConfirmationModal";
import { useSessionStore } from "./stores/sessionStore";
import { useSpriteStore } from "./stores/spriteStore";
import { useLayoutShortcuts } from "./hooks/useLayout";

function App() {
    const fetchSessions = useSessionStore((s) => s.fetchSessions);
    const listenForUpdates = useSessionStore((s) => s.listenForUpdates);
    const fetchSprites = useSpriteStore((s) => s.fetchSprites);

    // Register keyboard shortcuts
    useLayoutShortcuts();

    useEffect(() => {
        // Initialize data and event listeners on app start
        fetchSessions();
        listenForUpdates();
        fetchSprites();
    }, [fetchSessions, listenForUpdates, fetchSprites]);

    return (
        <>
            <AppShell />
            <ConfirmationModal />
        </>
    );
}

export default App;
