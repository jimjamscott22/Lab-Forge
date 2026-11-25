import { useEffect, useRef, useState } from "react";
import { Challenges } from "./views/Challenges";
import { Dashboard } from "./views/Dashboard";
import { Workspace } from "./views/Workspace";

export type Route =
  | { name: "dashboard" }
  | { name: "challenges" }
  | { name: "attempt"; attemptId: number };

const NAV = [
  { route: { name: "dashboard" }, label: "Dashboard" },
  { route: { name: "challenges" }, label: "Challenges" },
] as const satisfies readonly { route: Route; label: string }[];

export default function App() {
  const [route, setRoute] = useState<Route>({ name: "dashboard" });
  const main = useRef<HTMLElement>(null);
  const first = useRef(true);

  // After navigating, move focus to the main region so keyboard users land on the new view.
  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    main.current?.focus();
  }, [route]);

  const activeNav = route.name === "attempt" ? "challenges" : route.name;

  return (
    <div className="flex min-h-screen">
      <a
        href="#main"
        onClick={(e) => {
          e.preventDefault();
          main.current?.focus();
        }}
        className="sr-only focus:not-sr-only focus:absolute focus:left-2 focus:top-2 focus:z-10 focus:rounded focus:bg-moss-400 focus:px-3 focus:py-2 focus:text-ink-950"
      >
        Skip to content
      </a>

      <nav aria-label="Main" className="w-52 shrink-0 border-r border-ink-700 bg-ink-950 px-3 py-6">
        <p className="mb-6 px-3 text-xl font-bold">LabForge</p>
        <ul className="space-y-1">
          {NAV.map((item) => (
            <li key={item.route.name}>
              <button
                type="button"
                aria-current={activeNav === item.route.name ? "page" : undefined}
                onClick={() => setRoute(item.route)}
                className={`w-full rounded border-l-4 px-3 py-2 text-left ${
                  activeNav === item.route.name
                    ? "border-moss-400 bg-ink-800 font-bold"
                    : "border-transparent hover:bg-ink-800"
                }`}
              >
                {item.label}
              </button>
            </li>
          ))}
        </ul>
      </nav>

      <main id="main" ref={main} tabIndex={-1} className="min-w-0 flex-1 px-8 py-8 outline-none">
        {route.name === "dashboard" && <Dashboard navigate={setRoute} />}
        {route.name === "challenges" && <Challenges navigate={setRoute} />}
        {route.name === "attempt" && <Workspace attemptId={route.attemptId} navigate={setRoute} />}
      </main>
    </div>
  );
}
