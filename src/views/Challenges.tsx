import { useEffect, useState } from "react";
import { api, toAppError } from "../api/client";
import type { ChallengeSummary } from "../api/types";
import { difficultyLabel, plural } from "../lib/format";
import { useLoad } from "../lib/useLoad";
import type { Route } from "../App";

const SEARCH_DEBOUNCE_MS = 200;

export function Challenges({ navigate }: { navigate: (route: Route) => void }) {
  const [input, setInput] = useState("");
  const [query, setQuery] = useState("");
  const [startError, setStartError] = useState<{ id: number; message: string } | null>(null);

  useEffect(() => {
    const t = setTimeout(() => setQuery(input.trim()), SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(t);
  }, [input]);

  const { data, error, loading, reload } = useLoad(() => api.listChallenges(query), [query]);

  async function start(challenge: ChallengeSummary) {
    setStartError(null);
    try {
      const view = await api.startAttempt(challenge.id);
      navigate({ name: "attempt", attemptId: view.attempt.id });
    } catch (e) {
      setStartError({ id: challenge.id, message: toAppError(e).message });
      reload();
    }
  }

  return (
    <div className="max-w-3xl">
      <h1 className="mb-6 text-3xl font-bold">Challenges</h1>

      <label htmlFor="challenge-search" className="mb-1 block font-bold">
        Search challenges
      </label>
      <input
        id="challenge-search"
        type="search"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        placeholder="Try “dns”"
        className="mb-2 w-full max-w-md rounded border border-ink-600 bg-ink-800 px-3 py-2 placeholder:text-mist"
      />
      <p role="status" className="mb-4 text-sm text-mist">
        {data ? plural(data.length, "challenge") : loading ? "Loading…" : ""}
      </p>

      {error && (
        <div role="alert" className="mb-4">
          <p className="mb-2 text-clay-400">{error}</p>
          <button type="button" onClick={reload} className="rounded border border-ink-600 px-3 py-1.5 hover:bg-ink-800">
            Try again
          </button>
        </div>
      )}

      {data && data.length === 0 && !error && (
        <p className="max-w-[68ch] text-mist">
          {query ? `No challenges match “${query}”. Clear the search to see them all.` : "There are no challenges yet."}
        </p>
      )}

      {data && data.length > 0 && (
        <ul className="divide-y divide-ink-700 border-y border-ink-700">
          {data.map((c) => (
            <li key={c.id} className="py-5">
              <h2 className="text-xl font-bold">{c.title}</h2>
              <p className="mt-1 max-w-[68ch]">{c.summary}</p>
              <p className="mt-2 text-sm text-mist">
                {difficultyLabel[c.difficulty]} difficulty, {plural(c.stepCount, "step")}. {statusText(c)}
              </p>
              <div className="mt-3 flex flex-wrap gap-3">
                {c.latestAttemptStatus === "in_progress" && c.latestAttemptId !== null ? (
                  <ActionButton
                    primary
                    label={`Resume ${c.title}`}
                    onClick={() => navigate({ name: "attempt", attemptId: c.latestAttemptId as number })}
                  >
                    Resume
                  </ActionButton>
                ) : (
                  <ActionButton primary label={`${c.attemptCount ? "Start again" : "Start"}: ${c.title}`} onClick={() => void start(c)}>
                    {c.attemptCount ? "Start again" : "Start"}
                  </ActionButton>
                )}
                {c.latestAttemptStatus === "completed" && c.latestAttemptId !== null && (
                  <ActionButton
                    label={`Review last attempt: ${c.title}`}
                    onClick={() => navigate({ name: "attempt", attemptId: c.latestAttemptId as number })}
                  >
                    Review last attempt
                  </ActionButton>
                )}
              </div>
              {startError?.id === c.id && (
                <p role="alert" className="mt-2 text-clay-400">
                  {startError.message}
                </p>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function statusText(c: ChallengeSummary): string {
  if (c.latestAttemptStatus === "in_progress") return "In progress.";
  if (c.latestAttemptStatus === "completed") {
    return `Completed. ${plural(c.attemptCount, "attempt")} so far.`;
  }
  return "Not started.";
}

function ActionButton({
  children,
  label,
  onClick,
  primary = false,
}: {
  children: string;
  label: string;
  onClick: () => void;
  primary?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={
        primary
          ? "rounded bg-moss-400 px-4 py-2 font-bold text-ink-950 hover:bg-moss-300"
          : "rounded border border-ink-600 px-4 py-2 hover:bg-ink-800"
      }
    >
      {children}
    </button>
  );
}
