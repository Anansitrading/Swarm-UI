interface ContextBarProps {
    contextTokens: number;
    inputTokens: number;
    cacheCreationTokens?: number;
    cacheReadTokens?: number;
    model: string | null;
}

const MAX_CONTEXT: Record<string, number> = {
    opus: 200000,
    sonnet: 200000,
    haiku: 200000,
};

function getMaxContext(model: string | null): number {
    if (!model) return 200000;
    const lower = model.toLowerCase();
    for (const [key, val] of Object.entries(MAX_CONTEXT)) {
        if (lower.includes(key)) return val;
    }
    return 200000;
}

export function ContextBar({
    contextTokens,
    inputTokens,
    cacheCreationTokens,
    cacheReadTokens,
    model,
}: ContextBarProps) {
    const maxContext = getMaxContext(model);
    // Use contextTokens (input + cache_creation + cache_read) for the real context window usage
    const tokens = contextTokens > 0 ? contextTokens : inputTokens;
    const percentage = Math.min((tokens / maxContext) * 100, 100);

    const barColor =
        percentage > 80
            ? "bg-red-500"
            : percentage > 60
              ? "bg-yellow-500"
              : "bg-swarm-accent";

    const formatTokens = (n: number) => {
        if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
        if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
        return String(n);
    };

    // Build tooltip with breakdown
    const parts: string[] = [];
    if (inputTokens > 0) parts.push(`Input: ${formatTokens(inputTokens)}`);
    if (cacheCreationTokens && cacheCreationTokens > 0)
        parts.push(`Cache write: ${formatTokens(cacheCreationTokens)}`);
    if (cacheReadTokens && cacheReadTokens > 0)
        parts.push(`Cache read: ${formatTokens(cacheReadTokens)}`);
    const tooltip = parts.join(" | ");

    return (
        <div className="space-y-1" title={tooltip}>
            <div className="flex justify-between text-xs text-swarm-text-dim">
                <span>
                    Context: {formatTokens(tokens)} / {formatTokens(maxContext)}
                </span>
                <span>{percentage.toFixed(1)}%</span>
            </div>
            <div className="h-1.5 bg-swarm-border rounded-full overflow-hidden">
                <div
                    className={`h-full ${barColor} rounded-full transition-all duration-500`}
                    style={{ width: `${percentage}%` }}
                />
            </div>
        </div>
    );
}
