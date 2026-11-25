import { useId, useState } from "react";
import type { SaveState } from "./useNoteDraft";
import { Markdown } from "./Markdown";
import { formatDateTime } from "../lib/format";

interface Props {
  text: string;
  onChange: (value: string) => void;
  state: SaveState;
  onSave: () => void;
}

export function Notebook({ text, onChange, state, onSave }: Props) {
  const [preview, setPreview] = useState(false);
  const helpId = useId();

  const status =
    state.kind === "saving"
      ? "Saving…"
      : state.kind === "dirty"
        ? "Unsaved changes"
        : state.kind === "saved"
          ? state.at
            ? `Saved ${formatDateTime(state.at)}`
            : "Nothing to save yet"
          : null;

  return (
    <section aria-labelledby="notebook-heading">
      <div className="mb-2 flex items-baseline justify-between gap-2">
        <h2 id="notebook-heading" className="text-lg font-bold">
          Notebook
        </h2>
        <button
          type="button"
          aria-pressed={preview}
          onClick={() => setPreview((p) => !p)}
          className="rounded px-2 py-1 text-sm text-moss-300 underline underline-offset-4 hover:bg-ink-800"
        >
          {preview ? "Edit" : "Preview"}
        </button>
      </div>

      {preview ? (
        <div
          className="min-h-48 rounded border border-ink-700 bg-ink-800 p-3"
          aria-label="Notebook preview"
          role="region"
        >
          {text.trim() ? <Markdown source={text} /> : <p className="text-mist">Nothing written yet.</p>}
        </div>
      ) : (
        <>
          <label htmlFor="notebook-text" className="sr-only">
            Notebook
          </label>
          <textarea
            id="notebook-text"
            value={text}
            onChange={(e) => onChange(e.target.value)}
            aria-describedby={helpId}
            rows={10}
            spellCheck
            className="min-h-48 w-full resize-y rounded border border-ink-600 bg-ink-800 p-3 text-paper placeholder:text-mist"
            placeholder="Write what you observe. Cite packet numbers."
          />
        </>
      )}

      <p id={helpId} className="mt-2 text-sm text-mist">
        Markdown is supported. Notes belong to this attempt and save automatically.
      </p>

      <div className="mt-2 flex flex-wrap items-center gap-3">
        <button
          type="button"
          onClick={onSave}
          disabled={state.kind === "saving"}
          className="rounded border border-moss-600 px-3 py-1.5 text-moss-300 hover:bg-ink-800 disabled:opacity-60"
        >
          {state.kind === "error" ? "Try saving again" : "Save now"}
        </button>
        {status && (
          <span role="status" className="text-sm text-mist">
            {status}
          </span>
        )}
      </div>
      {state.kind === "error" && (
        <p role="alert" className="mt-2 rounded border border-clay-400 p-2 text-sm text-clay-400">
          Your notes were not saved. {state.message} Your text is still in the notebook.
        </p>
      )}
    </section>
  );
}
