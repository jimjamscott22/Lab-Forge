import type { AttemptStep, Step } from "../api/types";

interface Props {
  steps: Step[];
  progress: AttemptStep[];
  selectedStepId: number | null;
  onSelect: (stepId: number) => void;
}

/** Ordered step list. Done and current steps are reachable; later steps stay disabled. */
export function StepRail({ steps, progress, selectedStepId, onSelect }: Props) {
  const status = new Map(progress.map((p) => [p.stepId, p.status]));
  const currentId = steps.find((s) => status.get(s.id) !== "completed")?.id ?? null;

  return (
    <nav aria-label="Steps">
      <ol className="space-y-1">
        {steps.map((step, i) => {
          const done = status.get(step.id) === "completed";
          const current = step.id === currentId;
          const label = done ? "Done" : current ? "Current" : "Up next";
          const reachable = done || current;
          return (
            <li key={step.id}>
              <button
                type="button"
                disabled={!reachable}
                aria-current={step.id === selectedStepId ? "step" : undefined}
                onClick={() => onSelect(step.id)}
                className={`flex w-full flex-col gap-0.5 rounded border-l-4 px-3 py-2 text-left disabled:cursor-not-allowed disabled:opacity-60 ${
                  step.id === selectedStepId
                    ? "border-moss-400 bg-ink-700"
                    : "border-transparent hover:bg-ink-800"
                }`}
              >
                <span className="font-bold">
                  {i + 1}. {step.title}
                </span>
                <span className={`text-sm ${done ? "text-moss-300" : "text-mist"}`}>{label}</span>
              </button>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
