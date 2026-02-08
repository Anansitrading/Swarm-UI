interface ContextBarProps {
  inputTokens: number;
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

export function ContextBar({ inputTokens, model }: ContextBarProps) {
  const maxContext = getMaxContext(model);
  const percentage = Math.min((inputTokens / maxContext) * 100, 100);

  const barColor =
    percentage > 80
      ? "bg-red-500"
      : percentage > 60
        ? "bg-yellow-500"
        : "bg-swarm-accent";

  const formatTokens = (n: number) => {
    if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
    return String(n);
  };

  return (
    <div className="space-y-1">
      <div className="flex justify-between text-xs text-swarm-text-dim">
        <span>Context: {formatTokens(inputTokens)} / {formatTokens(maxContext)}</span>
        <span>{percentage.toFixed(0)}%</span>
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
