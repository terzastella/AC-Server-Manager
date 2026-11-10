// Verifica che la versione sia la stessa ovunque: package.json, package-lock.json,
// Cargo.toml, Cargo.lock e tauri.conf.json. Un disallineamento silenzioso produce
// binari con una versione, una release con un'altra e note che non corrispondono.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const read = (path) => readFileSync(path, "utf8");

const cargo = read(join(here, "..", "src-tauri", "Cargo.toml"));
const cargoLock = read(join(here, "..", "src-tauri", "Cargo.lock"));
const lock = JSON.parse(read(join(here, "..", "package-lock.json")));
const block = cargoLock
  .split("[[package]]")
  .find((item) => item.includes('name = "ac-server-control"'));

const versions = [
  ["package.json", JSON.parse(read(join(here, "..", "package.json"))).version],
  ["package-lock.json (root)", lock.version],
  ["package-lock.json (packages)", lock.packages?.[""]?.version],
  ["Cargo.toml", cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1]],
  ["Cargo.lock", block?.match(/version\s*=\s*"([^"]+)"/)?.[1]],
  ["tauri.conf.json", JSON.parse(read(join(here, "..", "src-tauri", "tauri.conf.json"))).version],
];

const expected = versions[0][1];
let errors = 0;
for (const [file, version] of versions) {
  if (!version) {
    console.error(`${file}: versione non trovata`);
    errors += 1;
    continue;
  }
  if (version !== expected) {
    console.error(`${file}: ${version} diversa da ${expected}`);
    errors += 1;
  }
}

console.log(`versione controllata: ${expected} in ${versions.length} punti`);
if (errors > 0) {
  console.error(`errori: ${errors}`);
  process.exit(1);
}
console.log("versioni allineate");
