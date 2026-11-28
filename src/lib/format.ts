import type { AttemptCard } from "../api/types";

const dateTime = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

export function formatDateTime(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso : dateTime.format(date);
}

export function plural(count: number, one: string, many = `${one}s`): string {
  return `${count} ${count === 1 ? one : many}`;
}

export const difficultyLabel = {
  beginner: "Beginner",
  intermediate: "Intermediate",
  advanced: "Advanced",
} as const;

/** Milliseconds as seconds with millisecond precision, e.g. 6312 -> "6.312". */
export function seconds(ms: number): string {
  return (ms / 1000).toFixed(3);
}

/** One-line status of an attempt, e.g. "In progress, 2 of 4 steps done". */
export function describeAttempt(card: AttemptCard): string {
  return card.status === "completed"
    ? `Completed ${card.completedAt ? formatDateTime(card.completedAt) : ""}`.trim()
    : `In progress, ${card.stepsCompleted} of ${plural(card.stepsTotal, "step")} done`;
}
