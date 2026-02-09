import { useEffect } from "react";
import { AppShell } from "./components/layout/AppShell";
import { useSessionStore } from "./stores/sessionStore";
import { useSpriteStore } from "./stores/spriteStore";
import { useLayoutShortcuts } from "./hooks/useLayout";

function App() {
    const startSessionWatcher = useSessionStore((s) => s.startWatcher);
    const fetchSprites = useSpriteStore((s) => s.fetchSprites);

    // Register keyboard shortcuts
    useLayoutShortcuts();

    useEffect(() => {
        // Initialize watchers and data on app start
        startSessionWatcher();
        fetchSprites();
    }, [startSessionWatcher, fetchSprites]);

    return <AppShell />;
}

export default App;
