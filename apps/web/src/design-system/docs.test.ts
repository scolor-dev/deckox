import { describe, expect, it } from "vitest";
import indexSource from "./components/index.ts?raw";
import readme from "./README.md?raw";
import docsPage from "../../../../docs/design-system.html?raw";

const sfcModules = import.meta.glob("./components/*/*.vue", { query: "?raw", import: "default", eager: true });
const tokenModules = import.meta.glob("./tokens.css", { query: "?raw", import: "default", eager: true });
const docModules = import.meta.glob("./components/*/*.md", { query: "?raw", import: "default", eager: true });

const CATEGORIES = ["actions", "forms", "feedback", "overlay", "navigation", "layout", "data"];
const SECTIONS = ["When to use", "When not to use", "API", "Examples", "Accessibility", "Tokens", "Gotchas", "Migration"];
const NO_EQUIVALENT = "No existing equivalent in the app.";

interface Sfc {
  name: string;
  folder: string;
  source: string;
}

const sfcs: Sfc[] = Object.entries(sfcModules).map(([path, source]) => {
  const match = /\.\/components\/(\w+)\/(\w+)\.vue$/.exec(path);
  if (!match) throw new Error(`unexpected component path: ${path}`);
  return { folder: match[1], name: match[2], source };
});
const folders = [...new Set(sfcs.map((sfc) => sfc.folder))].sort();
const docFor = (folder: string): string | undefined => {
  const entry = Object.entries(docModules).find(([path]) => path === `./components/${folder}/${folder}.md`);
  return entry?.[1];
};
const exported = new Set([...indexSource.matchAll(/export \{ default as (\w+) \} from "\.\/(\w+)\/(\w+)\.vue";/g)].map((m) => m[1]));
const tokensSource = Object.values(tokenModules).join("\n");
const definedTokens = new Set(tokensSource.match(/--[\w-]+(?=\s*:)/g));

const collapse = (text: string) => text.replace(/\s+/g, " ").trim();

function balanced(source: string, open: number): { inner: string; end: number } {
  const pairs: Record<string, string> = { "{": "}", "[": "]", "(": ")" };
  let depth = 0;
  for (let i = open; i < source.length; i += 1) {
    const ch = source[i];
    if (ch in pairs) depth += 1;
    else if (ch === "}" || ch === "]" || ch === ")") {
      depth -= 1;
      if (depth === 0) return { inner: source.slice(open + 1, i), end: i };
    }
  }
  throw new Error("unbalanced block");
}

function splitTopLevel(text: string, separator: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let quoted = false;
  let start = 0;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (ch === '"') quoted = !quoted;
    if (quoted) continue;
    if ("{[(<".includes(ch)) depth += 1;
    else if ("}])".includes(ch) || (ch === ">" && text[i - 1] !== "=")) depth -= 1;
    else if (ch === separator && depth === 0) {
      parts.push(text.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(text.slice(start));
  return parts.map(collapse).filter((part) => part !== "");
}

interface Prop {
  optional: boolean;
  type: string;
  fallback: string;
}

function parseProps(source: string): Map<string, Prop> {
  const props = new Map<string, Prop>();
  const marker = source.indexOf("defineProps<{");
  if (marker === -1) return props;
  const block = balanced(source, marker + "defineProps<".length);
  const defaults = new Map<string, string>();
  const after = source.slice(block.end);
  const withDefaultsIndex = source.lastIndexOf("withDefaults(", marker);
  if (withDefaultsIndex !== -1) {
    const open = after.indexOf("{", after.indexOf("(),"));
    const literal = balanced(after, open);
    for (const entry of splitTopLevel(literal.inner, ",")) {
      const match = /^(\w+)\s*:\s*([\s\S]+)$/.exec(entry);
      if (match) defaults.set(match[1], collapse(match[2]));
    }
  }
  for (const statement of splitTopLevel(block.inner, ";")) {
    const match = /^(\w+)(\?)?\s*:\s*([\s\S]+)$/.exec(statement);
    if (!match) throw new Error(`cannot parse prop: ${statement}`);
    props.set(match[1], { optional: match[2] === "?", type: collapse(match[3]), fallback: defaults.get(match[1]) ?? "" });
  }
  return props;
}

function parseEmits(source: string): Map<string, string> {
  const events = new Map<string, string>();
  const marker = source.indexOf("defineEmits<{");
  if (marker === -1) return events;
  const block = balanced(source, marker + "defineEmits<".length);
  for (const statement of splitTopLevel(block.inner, ";")) {
    const match = /^"?([\w:.-]+)"?\s*:\s*([\s\S]+)$/.exec(statement);
    if (!match) throw new Error(`cannot parse emit: ${statement}`);
    events.set(match[1], collapse(match[2]));
  }
  return events;
}

function parseSlots(source: string): Set<string> {
  const slots = new Set<string>();
  for (const match of source.matchAll(/<slot\b([^>]*)>/g)) {
    const dynamic = /:name="`([^`]+)`"/.exec(match[1])?.[1];
    const fixed = /(?<![:\w])name="([^"]+)"/.exec(match[1])?.[1];
    slots.add(dynamic?.replace(/\$\{[^}]+\}/g, "{key}") ?? fixed ?? "default");
  }
  return slots;
}

const parseTokens = (source: string) => new Set([...source.matchAll(/var\((--[\w-]+)/g)].map((m) => m[1]));

function frontmatter(doc: string): Record<string, string> {
  const match = /^---\n([\s\S]*?)\n---\n/.exec(doc);
  if (!match) throw new Error("missing frontmatter");
  const fields: Record<string, string> = {};
  for (const line of match[1].split("\n")) {
    const pair = /^(\w+):\s*(.*)$/.exec(line);
    if (pair) fields[pair[1]] = pair[2].trim();
  }
  return fields;
}

const parseList = (raw: string): string[] => {
  const inner = /^\[(.*)\]$/.exec(raw)?.[1];
  if (inner === undefined) throw new Error(`not a list: ${raw}`);
  return inner.split(",").map((item) => item.trim()).filter((item) => item !== "");
};

const unwrapCell = (cell: string) => cell.replace(/\\\|/g, "|").trim().replace(/^`([\s\S]*)`$/, "$1");

function tableRows(block: string, label: string): string[][] | null {
  const lines = block.split("\n");
  const start = lines.findIndex((line) => line.trim() === `**${label}**`);
  if (start === -1) return null;
  const rows: string[][] = [];
  for (let i = start + 1; i < lines.length; i += 1) {
    const line = lines[i].trim();
    if (line.startsWith("**")) break;
    if (line === "None.") return [];
    if (line.startsWith("|")) rows.push(line.split(/(?<!\\)\|/).slice(1, -1).map(unwrapCell));
  }
  return rows.slice(2);
}

const H2 = (doc: string) => [...doc.matchAll(/^## (.+)$/gm)].map((m) => m[1]);
const section = (doc: string, title: string): string => {
  const start = doc.indexOf(`\n## ${title}\n`);
  const rest = doc.slice(start + 1);
  const next = rest.indexOf("\n## ", 3);
  return next === -1 ? rest : rest.slice(0, next);
};

describe("components barrel", () => {
  it("exports every component exactly under its file name", () => {
    expect([...exported].sort()).toEqual(sfcs.map((sfc) => sfc.name).sort());
  });
});

describe("component source", () => {
  it.each(sfcs)("$name has no comments", ({ source }) => {
    expect(/\/\*|\*\/|^\s*\/\/|<!--/m.test(source)).toBe(false);
  });

  it.each(sfcs)("$name uses only defined tokens", ({ source }) => {
    const undefinedTokens = [...parseTokens(source)].filter((token) => !definedTokens.has(token));
    expect(undefinedTokens).toEqual([]);
  });
});

describe("component docs", () => {
  it("has a doc for every component folder and no orphan docs", () => {
    const docFolders = Object.keys(docModules).map((path) => /\.\/components\/(\w+)\//.exec(path)?.[1] ?? path);
    expect(docFolders.sort()).toEqual(folders);
  });

  it.each(folders)("%s doc matches its source", (folder) => {
    const doc = docFor(folder);
    if (doc === undefined) throw new Error(`missing doc for ${folder}`);
    const members = sfcs.filter((sfc) => sfc.folder === folder);

    const meta = frontmatter(doc);
    expect(Object.keys(meta)).toEqual(["name", "components", "category", "replaces", "related"]);
    expect(meta.name).toBe(folder);
    expect(parseList(meta.components).sort()).toEqual(members.map((m) => m.name).sort());
    expect(CATEGORIES).toContain(meta.category);
    for (const related of parseList(meta.related)) expect(exported).toContain(related);

    expect(doc).toContain(`\n# ${folder}\n`);
    expect(H2(doc)).toEqual(SECTIONS);

    const api = section(doc, "API");
    const apiBlocks = new Map(api.split("\n### ").slice(1).map((block) => [block.split("\n")[0].trim(), block]));
    expect([...apiBlocks.keys()].sort()).toEqual(members.map((m) => m.name).sort());

    for (const member of members) {
      const block = apiBlocks.get(member.name);
      if (block === undefined) throw new Error(`missing API block for ${member.name}`);

      const propRows = tableRows(block, "Props");
      const eventRows = tableRows(block, "Events");
      const slotRows = tableRows(block, "Slots");
      if (propRows === null || eventRows === null || slotRows === null) throw new Error(`${member.name}: Props, Events and Slots labels are all required`);

      const props = parseProps(member.source);
      expect(propRows.map((row) => row[0]).sort()).toEqual([...props.keys()].sort());
      for (const [name, type, fallback] of propRows) {
        const prop = props.get(name);
        if (prop === undefined) throw new Error(`${member.name}: unknown prop ${name}`);
        expect(type, `${member.name}.${name} type`).toBe(prop.type);
        const expected = prop.fallback !== "" ? prop.fallback : prop.optional ? "—" : "required";
        expect(fallback, `${member.name}.${name} default`).toBe(expected);
      }

      const events = parseEmits(member.source);
      expect(eventRows.map((row) => row[0]).sort()).toEqual([...events.keys()].sort());
      for (const [name, payload] of eventRows) expect(payload, `${member.name}.${name} payload`).toBe(events.get(name));

      expect(slotRows.map((row) => row[0]).sort()).toEqual([...parseSlots(member.source)].sort());
    }

    const documentedTokens = new Set(section(doc, "Tokens").match(/--[\w-]+/g));
    const usedTokens = new Set(members.flatMap((m) => [...parseTokens(m.source)]));
    expect([...documentedTokens].sort()).toEqual([...usedTokens].sort());

    const examples = section(doc, "Examples");
    expect(examples).toContain("```vue");
    for (const tag of examples.matchAll(/<([A-Z]\w+)/g)) expect(exported, `${folder} example uses ${tag[1]}`).toContain(tag[1]);

    const migration = section(doc, "Migration");
    if (parseList(meta.replaces).length === 0) {
      expect(migration.replace(/^## Migration\n/, "").trim()).toBe(NO_EQUIVALENT);
    } else {
      expect(migration.match(/```/g)?.length ?? 0).toBeGreaterThanOrEqual(4);
    }
  });
});

describe("docs/design-system.html", () => {
  it("lists every component folder", () => {
    const missing = folders.filter((folder) => !docsPage.includes(`<code>${folder}</code>`));
    expect(missing).toEqual([]);
  });
});

describe("design-system README", () => {
  it("links every component doc", () => {
    for (const folder of folders) expect(readme).toContain(`components/${folder}/${folder}.md`);
  });
});
