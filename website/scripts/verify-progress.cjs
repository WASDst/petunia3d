"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { validate } = require("../progress/progress.js");
const site = path.resolve(__dirname, "..");

try {
  const manifest = JSON.parse(fs.readFileSync(path.join(site, "docs/manifest.json"), "utf8"));
  const routes = new Set(["home.md", "about.md", ...manifest.domains.flatMap((domain) => domain.files.map((file) => file.path))]);
  const data = validate(JSON.parse(fs.readFileSync(path.join(site, "progress/tasks.json"), "utf8")), routes);
  const covered = new Set(data.tasks.flatMap((task) => task.documents));
  for (const route of routes) {
    const filename = path.resolve(site, "docs", route);
    if (!filename.startsWith(path.join(site, "docs") + path.sep) || !fs.statSync(filename).isFile()) throw new Error(`Documento inexistente ou inseguro: ${route}`);
    if (!covered.has(route)) throw new Error(`Documento sem processo associado: ${route}`);
  }
  const counts = Object.fromEntries(["TODO", "IN PROGRESS", "DONE"].map((status) => [status, data.tasks.filter((task) => task.status === status).length]));
  console.log(`PASS: ${data.tasks.length} processos, ${routes.size} documentos cobertos; ${JSON.stringify(counts)}; estados/checkpoints/evidências consistentes.`);
} catch (error) {
  console.error(`FAIL: acompanhamento da reimplementação — ${error.message}`);
  process.exitCode = 1;
}
