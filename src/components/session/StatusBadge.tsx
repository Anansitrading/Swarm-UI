import type { SessionStatus } from "../../types/session";
import {
  statusDisplayName,
  statusColor,
  statusDotColor,
  isActiveStatus,
} from "../../types/session";

interface StatusBadgeProps {
  status: SessionStatus;
}

export function StatusBadge({ status }: StatusBadgeProps) {
  const active = isActiveStatus(status);

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium ${statusColor(status)} bg-current/10 border border-current/20`}
    >
      <span className="relative flex h-1.5 w-1.5">
        <span className={`absolute inline-flex h-full w-full rounded-full ${statusDotColor(status)} ${active ? "animate-pulse-ring" : ""}`} />
        <span className={`relative inline-flex rounded-full h-1.5 w-1.5 ${statusDotColor(status)}`} />
      </span>
      {statusDisplayName(status)}
    </span>
  );
}
