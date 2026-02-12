interface HighlightTextProps {
    text: string;
    ranges: { start: number; end: number }[];
    className?: string;
    highlightClassName?: string;
}

export function HighlightText({
    text,
    ranges,
    className = "",
    highlightClassName = "bg-amber-400/80 text-gray-950 rounded-sm px-0.5 font-medium",
}: HighlightTextProps) {
    if (ranges.length === 0) {
        return <span className={className}>{text}</span>;
    }

    const parts: React.ReactNode[] = [];
    let lastEnd = 0;

    for (const { start, end } of ranges) {
        if (start > lastEnd) {
            parts.push(text.slice(lastEnd, start));
        }
        parts.push(
            <mark key={start} className={highlightClassName}>
                {text.slice(start, end)}
            </mark>,
        );
        lastEnd = end;
    }
    if (lastEnd < text.length) {
        parts.push(text.slice(lastEnd));
    }

    return <span className={className}>{parts}</span>;
}
