/**
 * The Windows widget's cards.
 *
 * The provider is C#, which cannot be built on a Mac, so these checks cover
 * what can be checked anywhere: that each Adaptive Card template expands
 * cleanly for every state, that nothing is left unbound, that every tap is a
 * `nankiv://` link and nothing else, and that the card data examples still
 * match the shared snapshot fixtures.
 *
 * The examples in cards/examples are the specification for `CardData.Build`;
 * the Windows test project holds the C# to them. If a state gains a field here,
 * that test fails on Windows until the provider follows.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { Template } from "adaptivecards-templating";

// Resolved from the project root: vitest serves module URLs, not file paths.
const here = join(process.cwd(), "widgets", "windows");
const cards = join(here, "cards");
const examples = join(cards, "examples");
const fixtures = join(here, "..", "fixtures");

const SIZES = ["small", "medium", "large"] as const;

function read(path: string): unknown {
  return JSON.parse(readFileSync(path, "utf8"));
}

function exampleNames(): string[] {
  return readdirSync(examples)
    .filter((f) => f.endsWith(".json"))
    .map((f) => f.replace(/\.json$/, ""))
    .sort();
}

function expand(size: string, data: unknown): Record<string, unknown> {
  const template = new Template(read(join(cards, `${size}.json`)));
  return template.expand({ $root: data }) as Record<string, unknown>;
}

/** Every string anywhere in the expanded card. */
function strings(node: unknown, out: string[] = []): string[] {
  if (typeof node === "string") out.push(node);
  else if (Array.isArray(node)) node.forEach((n) => strings(n, out));
  else if (node && typeof node === "object") {
    Object.values(node as Record<string, unknown>).forEach((n) =>
      strings(n, out),
    );
  }
  return out;
}

function walk(
  node: unknown,
  visit: (o: Record<string, unknown>) => void,
): void {
  if (Array.isArray(node)) node.forEach((n) => walk(n, visit));
  else if (node && typeof node === "object") {
    visit(node as Record<string, unknown>);
    Object.values(node as Record<string, unknown>).forEach((n) =>
      walk(n, visit),
    );
  }
}

describe("the card templates", () => {
  it("target the Adaptive Cards version Windows widgets support", () => {
    for (const size of SIZES) {
      const card = read(join(cards, `${size}.json`)) as Record<string, unknown>;
      expect(card.type).toBe("AdaptiveCard");
      expect(card.version).toBe("1.6");
    }
  });

  it("use only elements and styles the widget host renders", () => {
    // Adaptive Cards the Windows widget host supports, and the Fluent type
    // ramp from Microsoft's widget design guidance.
    const elements = new Set([
      "Container",
      "ColumnSet",
      "Column",
      "TextBlock",
      "FactSet",
      "Image",
    ]);
    const sizes = new Set([
      "Small",
      "Default",
      "Medium",
      "Large",
      "ExtraLarge",
    ]);
    const weights = new Set(["Lighter", "Default", "Bolder"]);
    for (const size of SIZES) {
      walk(read(join(cards, `${size}.json`)), (node) => {
        if (
          typeof node.type === "string" &&
          node.type !== "AdaptiveCard" &&
          !node.type.startsWith("Action.")
        ) {
          expect(elements, `${size}: ${node.type}`).toContain(node.type);
        }
        if (node.type === "TextBlock") {
          if (node.size)
            expect(sizes, `${size}: text size`).toContain(node.size);
          if (node.weight)
            expect(weights, `${size}: text weight`).toContain(node.weight);
        }
      });
    }
  });

  it("only ever open nankiv, never invoke the provider", () => {
    for (const size of SIZES) {
      walk(read(join(cards, `${size}.json`)), (node) => {
        if (typeof node.type === "string" && node.type.startsWith("Action.")) {
          // Action.Execute would let a card ask the provider to do something.
          expect(node.type, `${size}`).toBe("Action.OpenUrl");
          expect(String(node.url)).toMatch(/^(\$\{link\}|nankiv:\/\/)/);
        }
      });
    }
  });
});

describe("expanding every state", () => {
  const names = exampleNames();

  it("has an example for every shared fixture", () => {
    const fixtureNames = readdirSync(fixtures)
      .filter((f) => f.endsWith(".json") && f !== "links.json")
      .map((f) => f.replace(/\.json$/, ""));
    for (const fixture of fixtureNames) {
      expect(names, `no card example for the ${fixture} fixture`).toContain(
        fixture,
      );
    }
    // Plus the states that are not snapshots at all.
    expect(names).toContain("not_started");
    expect(names).toContain("needs_refresh");
    expect(names).toContain("removed");
  });

  for (const name of names) {
    for (const size of SIZES) {
      it(`${name} on ${size}`, () => {
        const data = read(join(examples, `${name}.json`)) as Record<
          string,
          unknown
        >;
        const card = expand(size, data);
        const texts = strings(card);

        // Nothing unbound, and no literal "undefined" or "null" on screen.
        for (const text of texts) {
          expect(text, `${name}/${size}`).not.toMatch(/\$\{/);
          expect(text, `${name}/${size}`).not.toMatch(/^(undefined|null)$/);
        }

        // Something to read, and somewhere to go.
        const body = card.body as unknown[];
        expect(body.length, `${name}/${size} has no body`).toBeGreaterThan(0);
        expect(strings(card.selectAction)).toContain(data.link);

        // Every TextBlock that survived has text.
        walk(card, (node) => {
          if (node.type === "TextBlock") {
            expect(
              String(node.text ?? "").trim(),
              `${name}/${size} empty TextBlock`,
            ).not.toBe("");
          }
        });
      });
    }
  }

  it("shows the answer, and calls a cutoff an estimate", () => {
    const data = read(join(examples, "ready.json")) as Record<string, unknown>;
    for (const size of SIZES) {
      const texts = strings(expand(size, data));
      expect(texts, size).toContain("You're in");
      expect(texts.join(" "), size).toContain("Aurora Systems");
    }
    const medium = strings(expand("medium", data)).join(" ");
    expect(medium).toContain("CGPA cutoff around 8.5");
    expect(medium).toContain("Estimate from all 96 students");
    expect(medium).not.toMatch(/official/i);
  });

  it("never turns an unknown answer into a rejection", () => {
    const data = read(join(examples, "undetermined.json")) as Record<
      string,
      unknown
    >;
    for (const size of SIZES) {
      const texts = strings(expand(size, data)).join(" ");
      expect(texts, size).toContain("Can't tell");
      expect(texts, size).not.toContain("Not this time");
    }
  });

  it("explains a failed import without showing the reason on the card", () => {
    const data = read(join(examples, "failed.json")) as Record<string, unknown>;
    expect(strings(expand("small", data)).join(" ")).toContain(
      "Last import failed",
    );
    expect(strings(expand("large", data)).join(" ")).toContain(
      "Couldn't import Round 2 - final.xlsx",
    );
  });

  it("gives each recent shortlist its own link on the large card", () => {
    const data = read(join(examples, "ready.json")) as Record<string, unknown>;
    const card = expand("large", data);
    const urls: string[] = [];
    walk(card, (node) => {
      if (node.type === "Action.OpenUrl" && typeof node.url === "string")
        urls.push(node.url);
    });
    const recent = data.recent as { link: string }[];
    for (const row of recent) expect(urls).toContain(row.link);
    expect(new Set(urls).size).toBe(recent.length + 1); // plus the card itself
  });

  it("names the people in your circle on the large card", () => {
    const data = read(join(examples, "ready.json")) as Record<string, unknown>;
    const texts = strings(expand("large", data)).join(" ");
    const circle = data.circle as { members: { label: string }[] };
    for (const member of circle.members) expect(texts).toContain(member.label);
  });

  it("shows one insight beside the answer on the medium card, not several", () => {
    const withAnalysis = strings(
      expand("medium", read(join(examples, "ready.json"))),
    ).join(" ");
    expect(withAnalysis).toContain("CGPA cutoff around 8.5");
    expect(withAnalysis).not.toContain("Your circle");

    const withoutAnalysis = strings(
      expand("medium", read(join(examples, "insufficient.json"))),
    ).join(" ");
    expect(withoutAnalysis).toContain("Your circle");
  });

  it("keeps the season and the recent list while resting", () => {
    const data = read(join(examples, "resting.json")) as Record<
      string,
      unknown
    >;
    const medium = strings(expand("medium", data)).join(" ");
    expect(medium).toContain("No new shortlist");
    expect(medium).toContain("Drop the next one into nankiv");
    expect(medium).toContain("3 in · 2 not in");
    expect(medium).toContain("Aurora Systems");

    // Each recent row still opens its own shortlist.
    const urls: string[] = [];
    walk(expand("large", data), (node) => {
      if (node.type === "Action.OpenUrl" && typeof node.url === "string")
        urls.push(node.url);
    });
    for (const row of data.recent as { link: string }[])
      expect(urls).toContain(row.link);
    // And the widget itself opens the app, not a shortlist.
    expect(urls).toContain("nankiv://shortlists");
  });

  it("carries no identifiers, only the labels a student typed", () => {
    for (const name of names) {
      const raw = readFileSync(join(examples, `${name}.json`), "utf8");
      expect(raw, name).not.toMatch(/[A-Z]\d[A-Z]\d[A-Z]\d[A-Z]\d/); // Neo ID shape
      expect(raw, name).not.toMatch(/\d{2}[A-Z]{3}\d{4}/); // registration number shape
    }
  });
});
