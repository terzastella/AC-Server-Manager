// Verifica che i dizionari IT/EN abbiano le stesse chiavi e gli stessi segnaposto.
// Uso: node scripts/check-i18n.mjs  (eseguito anche in CI)
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const dir = join(here, "..", "src", "i18n", "namespaces");
const files = readdirSync(dir).filter((name) => name.endsWith(".ts"));

const placeholders = (value) =>
  [...value.matchAll(/\{(\w+)\}/g)]
    .map((match) => match[1])
    .sort()
    .join(",");

const entries = (block) =>
  [...block.matchAll(/"([^"]+)"\s*:\s*"([^"]*)"/g)].map((match) => ({
    key: match[1],
    value: match[2],
  }));

let errors = 0;

for (const file of files) {
  const source = readFileSync(join(dir, file), "utf8");
  const [itBlock, enBlock = ""] = source.split("  en: {");

  const it = new Map(entries(itBlock).map((entry) => [entry.key, entry.value]));
  const en = new Map(entries(enBlock).map((entry) => [entry.key, entry.value]));

  for (const key of it.keys()) {
    if (!en.has(key)) {
      console.error(`${file}: manca in EN -> ${key}`);
      errors += 1;
    }
  }
  for (const key of en.keys()) {
    if (!it.has(key)) {
      console.error(`${file}: chiave solo in EN -> ${key}`);
      errors += 1;
    }
  }
  for (const [key, value] of it) {
    if (!en.has(key)) continue;
    const expected = placeholders(value);
    const actual = placeholders(en.get(key));
    if (expected !== actual) {
      console.error(
        `${file}: segnaposto diversi in "${key}" (it: ${expected || "-"}, en: ${actual || "-"})`,
      );
      errors += 1;
    }
  }
}

console.log(`namespace controllati: ${files.length}`);
if (errors > 0) {
  console.error(`errori: ${errors}`);
  process.exit(1);
}
console.log("i18n ok: chiavi e segnaposto allineati");
