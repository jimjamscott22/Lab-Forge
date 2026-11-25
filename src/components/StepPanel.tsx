import { useEffect, useRef, useState } from "react";
import type { Step } from "../api/types";
import { Markdown } from "./Markdown";

export interface CheckFeedback {
  correct: boolean;
  explanation: string;
}

interface Props {
  step: Step;
  position: number;
  total: number;
  completed: boolean;
  selectedChoiceId: number | null;
  busy: boolean;
  feedback: CheckFeedback | null;
  error: string | null;
  /** Focus the heading on mount. Set once the learner has moved between steps. */
  focusOnMount: boolean;
  onSubmit: (choiceId: number | null) => void;
}

const actionLabel = { task: "Mark step done", check: "Check answer", note: "Complete step" } as const;

/** Instructions, on-request hints, and the one action that completes the step. */
export function StepPanel({
  step,
  position,
  total,
  completed,
  selectedChoiceId,
  busy,
  feedback,
  error,
  focusOnMount,
  onSubmit,
}: Props) {
  const [hintsShown, setHintsShown] = useState(0);
  const [choice, setChoice] = useState<number | null>(selectedChoiceId);
  const heading = useRef<HTMLHeadingElement>(null);

  // The parent remounts this panel for every step (key = step id), so "on mount" means
  // "the learner arrived at a new step". Skipped for the very first step shown.
  useEffect(() => {
    if (focusOnMount) heading.current?.focus();
  }, [focusOnMount]);

  const canSubmit = !completed && !busy && (step.kind !== "check" || choice !== null);

  return (
    <article aria-labelledby="step-heading">
      <p className="text-sm text-mist">
        Step {position} of {total}
      </p>
      <h2 id="step-heading" ref={heading} tabIndex={-1} className="mb-3 text-2xl font-bold outline-offset-4">
        {step.title}
      </h2>

      <Markdown source={step.instructionsMd} />

      <p className="mt-4 max-w-[68ch]">
        <strong>You are done when:</strong> {step.completionCriteria}
      </p>

      {step.hints.length > 0 && (
        <div className="mt-5">
          {hintsShown > 0 && (
            <ul className="mb-3 space-y-2" aria-label="Hints">
              {step.hints.slice(0, hintsShown).map((hint) => (
                <li key={hint.position} className="max-w-[68ch] rounded border-l-4 border-moss-600 bg-ink-800 px-3 py-2">
                  <p className="text-sm font-bold text-moss-300">Hint {hint.position}</p>
                  <Markdown source={hint.bodyMd} />
                </li>
              ))}
            </ul>
          )}
          {hintsShown < step.hints.length && (
            <button
              type="button"
              onClick={() => setHintsShown((n) => n + 1)}
              className="rounded border border-ink-600 px-3 py-1.5 hover:bg-ink-800"
            >
              Show hint {hintsShown + 1} of {step.hints.length}
            </button>
          )}
        </div>
      )}

      {step.kind === "check" && (
        <fieldset className="mt-6 max-w-[68ch]" disabled={completed || busy}>
          <legend className="mb-2 font-bold">Your answer</legend>
          <div className="space-y-2">
            {step.choices.map((c) => (
              <label
                key={c.id}
                className="flex cursor-pointer items-start gap-3 rounded border border-ink-700 bg-ink-800 p-3 has-[:checked]:border-moss-400"
              >
                <input
                  type="radio"
                  name={`step-${step.id}`}
                  value={c.id}
                  checked={choice === c.id}
                  onChange={() => setChoice(c.id)}
                  className="mt-1 accent-moss-400"
                />
                <span>{c.label}</span>
              </label>
            ))}
          </div>
        </fieldset>
      )}

      {step.kind === "note" && !completed && (
        <p className="mt-4 max-w-[68ch] text-mist">
          Your notebook is saved before this step completes.
        </p>
      )}

      <div className="mt-6 flex flex-wrap items-center gap-3">
        {completed ? (
          <p className="font-bold text-moss-300">This step is done.</p>
        ) : (
          <button
            type="button"
            disabled={!canSubmit}
            onClick={() => onSubmit(step.kind === "check" ? choice : null)}
            className="rounded bg-moss-400 px-4 py-2 font-bold text-ink-950 hover:bg-moss-300 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {busy ? "Working…" : actionLabel[step.kind]}
          </button>
        )}
      </div>

      <div aria-live="polite" className="mt-3 max-w-[68ch]">
        {feedback && (
          <p className={feedback.correct ? "text-moss-300" : "text-clay-400"}>
            <strong>{feedback.correct ? "Correct." : "Not quite."}</strong> {feedback.explanation}
          </p>
        )}
      </div>
      {error && (
        <p role="alert" className="mt-3 max-w-[68ch] rounded border border-clay-400 p-2 text-clay-400">
          {error}
        </p>
      )}
    </article>
  );
}
