import { api } from "../api/client";
import type { AttemptCard } from "../api/types";
import { ProgressSegments } from "../components/ProgressSegments";
import { formatDateTime, plural } from "../lib/format";
import { useLoad } from "../lib/useLoad";
import type { Route } from "../App";

export function Dashboard({ navigate }: { navigate: (route: Route) => void }) {
  const { data, error, loading, reload } = useLoad(() => api.getDashboard(), []);

  if (loading && !data) return <p role="status">Loading dashboard…</p>;
  if (error || !data) {
    return (
      <div role="alert" className="max-w-[68ch]">
        <p className="mb-3 text-clay-400">{error ?? "The dashboard could not be loaded."}</p>
        <button type="button" onClick={reload} className="rounded border border-ink-600 px-3 py-1.5 hover:bg-ink-800">
          Try again
        </button>
      </div>
    );
  }

  const { current, recent } = data;
  return (
    <div className="max-w-3xl">
      <h1 className="mb-6 text-3xl font-bold">Dashboard</h1>

      {current ? (
        <section aria-labelledby="resume-heading" className="mb-8 rounded border border-moss-600 bg-ink-800 p-5">
          <h2 id="resume-heading" className="text-lg font-bold">
            Continue your attempt
          </h2>
          <p className="mt-1 text-xl">{current.challengeTitle}</p>
          <p className="mt-1 text-sm text-mist">Started {formatDateTime(current.startedAt)}</p>
          <div className="mt-3">
            <ProgressSegments done={current.stepsCompleted} total={current.stepsTotal} />
          </div>
          <button
            type="button"
            onClick={() => navigate({ name: "attempt", attemptId: current.attemptId })}
            className="mt-4 rounded bg-moss-400 px-4 py-2 font-bold text-ink-950 hover:bg-moss-300"
          >
            Resume
          </button>
        </section>
      ) : (
        <section aria-labelledby="start-heading" className="mb-8">
          <h2 id="start-heading" className="text-lg font-bold">
            Start an exercise
          </h2>
          <p className="mt-1 max-w-[68ch] text-mist">You have no attempt in progress. Pick a challenge to begin.</p>
          <button
            type="button"
            onClick={() => navigate({ name: "challenges" })}
            className="mt-3 rounded bg-moss-400 px-4 py-2 font-bold text-ink-950 hover:bg-moss-300"
          >
            Browse challenges
          </button>
        </section>
      )}

      <section aria-labelledby="progress-heading" className="mb-8">
        <h2 id="progress-heading" className="text-lg font-bold">
          Your progress
        </h2>
        <p className="mt-1">
          {data.challengesCompleted} of {plural(data.challengesAvailable, "challenge")} completed,{" "}
          {plural(data.attemptsCompleted, "attempt")} finished.
        </p>
      </section>

      <section aria-labelledby="recent-heading">
        <h2 id="recent-heading" className="mb-2 text-lg font-bold">
          Recent attempts
        </h2>
        {recent.length === 0 ? (
          <p className="text-mist">Attempts you start will show up here.</p>
        ) : (
          <ul className="divide-y divide-ink-700 border-y border-ink-700">
            {recent.map((card) => (
              <li key={card.attemptId} className="flex flex-wrap items-center justify-between gap-3 py-3">
                <div>
                  <p className="font-bold">{card.challengeTitle}</p>
                  <p className="text-sm text-mist">{describe(card)}</p>
                </div>
                <button
                  type="button"
                  onClick={() => navigate({ name: "attempt", attemptId: card.attemptId })}
                  aria-label={`Open attempt: ${card.challengeTitle}, ${describe(card)}`}
                  className="rounded border border-ink-600 px-3 py-1.5 hover:bg-ink-800"
                >
                  Open
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

function describe(card: AttemptCard): string {
  return card.status === "completed"
    ? `Completed ${card.completedAt ? formatDateTime(card.completedAt) : ""}`.trim()
    : `In progress, ${card.stepsCompleted} of ${plural(card.stepsTotal, "step")} done`;
}
