import { useEffect, useRef, useState } from "react";
import { api, toAppError } from "../api/client";
import type { AttemptView } from "../api/types";
import { Markdown } from "../components/Markdown";
import { Notebook } from "../components/Notebook";
import { PacketLedger } from "../components/PacketLedger";
import { ProgressSegments } from "../components/ProgressSegments";
import { StepPanel, type CheckFeedback } from "../components/StepPanel";
import { StepRail } from "../components/StepRail";
import { useNoteDraft } from "../components/useNoteDraft";
import { difficultyLabel, formatDateTime } from "../lib/format";
import { useLoad } from "../lib/useLoad";
import type { Route } from "../App";

interface Props {
  attemptId: number;
  navigate: (route: Route) => void;
}

export function Workspace({ attemptId, navigate }: Props) {
  const { data, error, loading, reload } = useLoad(() => api.getAttempt(attemptId), [attemptId]);

  if (loading && !data) return <p role="status">Loading attempt…</p>;
  if (error || !data) {
    return (
      <div role="alert" className="max-w-[68ch]">
        <p className="mb-3 text-clay-400">{error ?? "This attempt could not be loaded."}</p>
        <button type="button" onClick={reload} className="rounded border border-ink-600 px-3 py-1.5 hover:bg-ink-800">
          Try again
        </button>
      </div>
    );
  }
  return <WorkspaceLoaded key={attemptId} initial={data} navigate={navigate} />;
}

function WorkspaceLoaded({ initial, navigate }: { initial: AttemptView; navigate: (route: Route) => void }) {
  const [view, setView] = useState(initial);
  const { attempt, challenge, steps: progress } = view;
  const note = useNoteDraft(attempt.id, view.note?.bodyMd ?? "", view.note?.updatedAt ?? null);

  const statusOf = (stepId: number) => progress.find((p) => p.stepId === stepId);
  const currentStep = challenge.steps.find((s) => statusOf(s.id)?.status !== "completed") ?? null;
  const finished = attempt.status === "completed";

  const [selectedId, setSelectedId] = useState<number | null>(currentStep?.id ?? null);
  const [busy, setBusy] = useState(false);
  const [feedback, setFeedback] = useState<CheckFeedback | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [restartError, setRestartError] = useState<string | null>(null);
  // Focus follows the learner only after they act, never on first load (App focuses <main> then).
  const interacted = useRef(false);
  const [justFinished, setJustFinished] = useState(false);
  const summaryHeading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (justFinished) summaryHeading.current?.focus();
  }, [justFinished]);

  const selected = challenge.steps.find((s) => s.id === selectedId) ?? currentStep;
  const doneCount = progress.filter((p) => p.status === "completed").length;

  async function submit(choiceId: number | null) {
    if (!selected || busy) return;
    setBusy(true);
    setActionError(null);
    setFeedback(null);
    try {
      // The note step is judged against the saved note, so persist the draft first.
      if (selected.kind === "note") {
        const saved = await note.save();
        if (!saved) {
          setActionError("Save your notebook before completing this step. The save failed; try again.");
          return;
        }
      }
      const outcome = await api.completeStep(attempt.id, selected.id, choiceId);
      if (selected.kind === "check" && outcome.explanation) {
        setFeedback({ correct: outcome.correct, explanation: outcome.explanation });
      }
      if (outcome.correct) {
        interacted.current = true;
        setJustFinished(outcome.attempt.attempt.status === "completed");
        setView(outcome.attempt);
        const next = outcome.attempt.challenge.steps.find(
          (s) => outcome.attempt.steps.find((p) => p.stepId === s.id)?.status !== "completed",
        );
        setSelectedId(next?.id ?? null);
      }
    } catch (e) {
      setActionError(toAppError(e).message);
    } finally {
      setBusy(false);
    }
  }

  async function startAnother() {
    setRestartError(null);
    try {
      const next = await api.startAttempt(challenge.id);
      navigate({ name: "attempt", attemptId: next.attempt.id });
    } catch (e) {
      setRestartError(toAppError(e).message);
    }
  }

  return (
    <div>
      <header className="mb-6">
        <button
          type="button"
          onClick={() => navigate({ name: "challenges" })}
          className="mb-2 rounded px-1 text-moss-300 underline underline-offset-4 hover:bg-ink-800"
        >
          Back to challenges
        </button>
        <h1 className="text-3xl font-bold">{challenge.title}</h1>
        <p className="mt-1 text-mist">
          {difficultyLabel[challenge.difficulty]} difficulty. Attempt started {formatDateTime(attempt.startedAt)}.
        </p>
        <div className="mt-3">
          <ProgressSegments done={doneCount} total={challenge.steps.length} />
        </div>
      </header>

      <div className="grid gap-8 lg:grid-cols-[15rem_minmax(0,1fr)] xl:grid-cols-[15rem_minmax(0,1fr)_22rem]">
        <StepRail
          steps={challenge.steps}
          progress={progress}
          selectedStepId={finished ? null : (selected?.id ?? null)}
          onSelect={(id) => {
            interacted.current = true;
            setSelectedId(id);
            setFeedback(null);
            setActionError(null);
          }}
        />

        <div className="min-w-0">
          {finished || !selected ? (
            <section aria-labelledby="summary-heading">
              <h2 id="summary-heading" ref={summaryHeading} tabIndex={-1} className="mb-1 text-2xl font-bold outline-offset-4">
                Attempt complete
              </h2>
              <p className="mb-4 text-mist">
                Finished {attempt.completedAt ? formatDateTime(attempt.completedAt) : ""}. All{" "}
                {challenge.steps.length} steps are done.
              </p>
              <h3 className="mb-2 text-lg font-bold">Your notes</h3>
              {note.text.trim() ? (
                <div className="rounded border border-ink-700 bg-ink-800 p-4">
                  <Markdown source={note.text} />
                </div>
              ) : (
                <p className="text-mist">No notes were saved for this attempt.</p>
              )}
              <div className="mt-6 flex flex-wrap gap-3">
                <button
                  type="button"
                  onClick={() => void startAnother()}
                  className="rounded bg-moss-400 px-4 py-2 font-bold text-ink-950 hover:bg-moss-300"
                >
                  Start another attempt
                </button>
                <button
                  type="button"
                  onClick={() => navigate({ name: "history", challengeId: attempt.challengeId })}
                  className="rounded border border-ink-600 px-4 py-2 hover:bg-ink-800"
                >
                  View attempt history
                </button>
                <button
                  type="button"
                  onClick={() => navigate({ name: "dashboard" })}
                  className="rounded border border-ink-600 px-4 py-2 hover:bg-ink-800"
                >
                  Go to dashboard
                </button>
              </div>
              {restartError && (
                <p role="alert" className="mt-3 text-clay-400">
                  {restartError}
                </p>
              )}
            </section>
          ) : (
            <StepPanel
              key={selected.id}
              step={selected}
              position={selected.position}
              total={challenge.steps.length}
              completed={statusOf(selected.id)?.status === "completed"}
              selectedChoiceId={statusOf(selected.id)?.selectedChoiceId ?? null}
              busy={busy}
              feedback={feedback}
              error={actionError}
              focusOnMount={interacted.current}
              onSubmit={(choiceId) => void submit(choiceId)}
            />
          )}

          {challenge.evidence.map((e) => (
            <PacketLedger key={e.evidenceKey} evidence={e} />
          ))}
        </div>

        {/* Below xl the notebook drops under the exercise instead of squeezing it. */}
        <div className="min-w-0 lg:col-span-2 xl:col-span-1">
          <Notebook text={note.text} onChange={note.setText} state={note.state} onSave={() => void note.save()} />
        </div>
      </div>
    </div>
  );
}
