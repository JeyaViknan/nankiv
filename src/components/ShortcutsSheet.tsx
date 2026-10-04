/**
 * Every keyboard shortcut, on one page — Help → Keyboard Shortcuts, or ⌘/.
 *
 * Written the way the platform writes keys (⌘ on a Mac, Ctrl elsewhere), and
 * grouped by where they work, so the list answers "what can I do here?".
 */

import { MOD, chord } from "../lib/keys";
import { Sheet } from "./Sheet";

const GROUPS: { title: string; keys: [string, string][] }[] = [
  {
    title: "Anywhere",
    keys: [
      ["Open a shortlist", chord("O")],
      ["Paste a list of IDs", chord("V")],
      ["Import a reference sheet", chord("O", true)],
      ["Search a name or Neo ID", chord("F")],
      ["Your circle", chord("D")],
      ["Settings", chord(",")],
      ["Keyboard shortcuts", chord("/")],
    ],
  },
  {
    title: "Your shortlists",
    keys: [
      ["Move through the list", "↑ ↓"],
      ["Open the highlighted one", "Return"],
      ["Remove the highlighted one", MOD === "⌘" ? "⌘ Delete" : "Delete"],
    ],
  },
  {
    title: "A shortlist",
    keys: [
      ["Back to your shortlists", `${chord("[")}  or  Esc`],
      ["Download names as Excel", chord("E")],
      ["Download names as CSV", chord("E", true)],
    ],
  },
];

export function ShortcutsSheet({ onClose }: { onClose: () => void }) {
  return (
    <Sheet title="Keyboard shortcuts" onClose={onClose} width={480}>
      {GROUPS.map((g) => (
        <section className="keys-group" key={g.title}>
          <h2>{g.title}</h2>
          <dl className="keys">
            {g.keys.map(([what, keys]) => (
              <div className="keys-row" key={what}>
                <dt>{what}</dt>
                <dd>
                  <kbd>{keys}</kbd>
                </dd>
              </div>
            ))}
          </dl>
        </section>
      ))}
    </Sheet>
  );
}
