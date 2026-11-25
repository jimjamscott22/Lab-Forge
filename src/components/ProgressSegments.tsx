import { plural } from "../lib/format";

/** One block per step. Status is always spelled out in text next to it; color is secondary. */
export function ProgressSegments({ done, total }: { done: number; total: number }) {
  const label = `${done} of ${plural(total, "step")} done`;
  return (
    <div className="flex items-center gap-3">
      <div role="img" aria-label={label} className="flex w-40 gap-1">
        {Array.from({ length: total }, (_, i) => (
          <span
            key={i}
            className={`h-2 flex-1 rounded-sm ${i < done ? "bg-moss-400" : "bg-ink-600"}`}
          />
        ))}
      </div>
      <span className="text-sm text-mist">{label}</span>
    </div>
  );
}
