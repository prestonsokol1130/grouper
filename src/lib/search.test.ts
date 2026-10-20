import { describe, expect, it } from "vitest";
import { indexItems, indexName, initials, normalize, rank, SCORE, scoreName } from "./search";

const score = (name: string, query: string) => scoreName(indexName(name), normalize(query));

/** Ranks plain names and returns them in result order. */
function names(query: string, list: readonly string[]): string[] {
  return rank(query, indexItems(list.map((name) => ({ value: name, name }))));
}

describe("normalize", () => {
  it("drops case, accents and extra spaces", () => {
    expect(normalize("  Café   Crème ")).toBe("cafe creme");
    expect(normalize("ÅNGSTRÖM")).toBe("angstrom");
    expect(normalize("Señor")).toBe("senor");
  });

  it("gives initials per word, camelCase and accents included", () => {
    expect(initials("Visual Studio Code")).toBe("vsc");
    expect(initials("OneDrive")).toBe("od");
    expect(initials("Éditeur de Texte")).toBe("edt");
  });
});

describe("scoreName", () => {
  it("scores exact > prefix > word prefix > initials > substring", () => {
    expect(score("Notepad", "notepad")).toBe(SCORE.exact);
    expect(score("Notepad", "note")).toBe(SCORE.prefix);
    expect(score("Visual Studio Code", "stu")).toBe(SCORE.wordPrefix);
    expect(score("Visual Studio Code", "studio co")).toBe(SCORE.wordPrefix);
    expect(score("OneDrive", "dri")).toBe(SCORE.wordPrefix);
    expect(score("Visual Studio Code", "vsc")).toBe(SCORE.initials);
    expect(score("Notepad", "pad")).toBe(SCORE.substring);
    expect(score("Notepad", "paint")).toBe(SCORE.none);
    expect(score("Notepad", "")).toBe(SCORE.none);
  });

  it("ignores case and accents on both sides", () => {
    expect(score("Café", "CAFE")).toBe(SCORE.exact);
    expect(score("Cafe", "café")).toBe(SCORE.exact);
    expect(score("Paint 3D", "pAiNt")).toBe(SCORE.prefix);
  });
});

describe("rank", () => {
  it("orders by score, then by name", () => {
    const list = ["Paint", "Notepad++", "Notepad", "Snipping Tool", "Notes", "Sticky Notes"];
    expect(names("note", list)).toEqual(["Notepad", "Notepad++", "Notes", "Sticky Notes"]);
    expect(names("notepad", list)).toEqual(["Notepad", "Notepad++"]);
    expect(names("p", list)).toEqual(["Paint", "Notepad", "Notepad++", "Snipping Tool"]);
  });

  it("finds Visual Studio Code by vsc, after any name that starts with it", () => {
    const list = ["Visual Studio Code", "vscode-helper", "Microsoft Visual Studio Code"];
    expect(names("vsc", list)).toEqual([
      "vscode-helper",
      "Microsoft Visual Studio Code",
      "Visual Studio Code",
    ]);
  });

  it("matches nothing for an empty query", () => {
    expect(names("   ", ["Anything"])).toEqual([]);
  });

  it("matches a folder through its apps and returns the folder", () => {
    const items = indexItems([
      { value: "folder:dev", name: "Dev", members: ["Git Bash", "Bash Shell"] },
      { value: "app:bash", name: "Bash Helper" },
      { value: "app:other", name: "Other" },
    ]);
    expect(rank("git", items)).toEqual(["folder:dev"]);
    // Same level (prefix): the app's own name beats the folder's member.
    expect(rank("bash", items)).toEqual(["app:bash", "folder:dev"]);
    expect(rank("dev", items)).toEqual(["folder:dev"]);
  });

  it("ranks 1,000 items in under 2 ms", () => {
    const words = ["Studio", "Code", "Paint", "Note", "Edge", "Chrome", "Mail", "Café", "Tool"];
    const items = indexItems(
      Array.from({ length: 1000 }, (_, i) => ({
        value: i,
        name: `${words[i % 9] ?? ""} ${words[(i * 7) % 9] ?? ""} App ${i}`,
        members: i % 50 === 0 ? ["Visual Studio Code", "Notepad", "Calculator"] : undefined,
      })),
    );
    const queries = ["vsc", "cafe", "st", "app 99", "zzz", "c"];
    for (const query of queries) rank(query, items); // warm up the JIT
    const times: number[] = [];
    for (let run = 0; run < 20; run++) {
      for (const query of queries) {
        const start = performance.now();
        rank(query, items);
        times.push(performance.now() - start);
      }
    }
    times.sort((a, b) => a - b);
    const median = times[Math.floor(times.length / 2)] ?? Infinity;
    console.info(`rank over 1,000 items: median ${median.toFixed(3)} ms`);
    expect(median).toBeLessThan(2);
  });
});
