import { api } from "../api/client";
import { describeAttempt, formatDateTime } from "../lib/format";
import { useLoad } from "../lib/useLoad";
import type { Route } from "../App";

/** Must match `HISTORY_LIMIT` in `src-tauri/src/services/attempts.rs`. */
export const HISTORY_LIMIT = 200;

const secondaryButton = "rounded border border-ink-600 px-3 py-1.5 hover:bg-ink-800";

export function History({ challengeId, navigate }: { challengeId?: number; navigate: (route: Route) => void }) {
  const { data, error, loading, reload } = useLoad(() => api.listAttempts(challengeId), [challengeId]);
  const filtered = challengeId !== undefined;
  const showAll = (
    <button type="button" onClick={() => navigate({ name: "history" })} className={secondaryButton}>
      Show all attempts
    </button>
  );

  if (loading && !data) return <p role="status">Loading attempt history…</p>;
  if (error || !data) {
    return (
      <div role="alert" className="max-w-[68ch]">
        <p className="mb-3 text-clay-400">{error ?? "The attempt history could not be loaded."}</p>
        <div className="flex flex-wrap gap-3">
          <button type="button" onClick={reload} className={secondaryButton}>
            Try again
          </button>
          {filtered && showAll}
        </div>
      </div>
    );
  }

  const first = data[0];
  const title = filtered && first ? `Attempt history: ${first.challengeTitle}` : "Attempt history";
  return (
    <div className="max-w-3xl">
      <h1 className="mb-6 text-3xl font-bold">{title}</h1>
      {filtered && <div className="mb-6">{showAll}</div>}

      {data.length === 0 ? (
        <section>
          <p className="max-w-[68ch] text-mist">You have not started any attempts yet.</p>
          <button
            type="button"
            onClick={() => navigate({ name: "challenges" })}
            className="mt-3 rounded bg-moss-400 px-4 py-2 font-bold text-ink-950 hover:bg-moss-300"
          >
            Browse challenges
          </button>
        </section>
      ) : (
        <>
          {data.length >= HISTORY_LIMIT && (
            <p className="mb-3 text-mist">Showing the {HISTORY_LIMIT} most recent attempts.</p>
          )}
          <ul className="divide-y divide-ink-700 border-y border-ink-700">
            {data.map((card) => (
              <li key={card.attemptId} className="flex flex-wrap items-center justify-between gap-3 py-3">
                <div>
                  <p className="font-bold">{card.challengeTitle}</p>
                  <p className="text-sm">{describeAttempt(card)}</p>
                  <p className="text-sm text-mist">
                    Version {card.challengeVersion} · Started {formatDateTime(card.startedAt)}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => navigate({ name: "attempt", attemptId: card.attemptId })}
                  aria-label={`Open attempt: ${card.challengeTitle}, ${describeAttempt(card)}`}
                  className={secondaryButton}
                >
                  Open
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}
