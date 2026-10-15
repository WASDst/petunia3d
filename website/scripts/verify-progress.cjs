"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { validate } = require("../progress/progress.js");
const site = path.resolve(__dirname, "..");

try {
  const manifest = JSON.parse(fs.readFileSync(path.join(site, "docs/manifest.json"), "utf8"));
  const routes = new Set(["home.md", "about.md", ...manifest.domains.flatMap((domain) => domain.files.map((file) => file.path))]);
  const data = validate(JSON.parse(fs.readFileSync(path.join(site, "progress/tasks.json"), "utf8")), routes);
  const matrixDocument = "02-application/tools-interaction-implementation.md";
  const matrixText = fs.readFileSync(path.join(site, "docs", matrixDocument), "utf8");
  const matrix = matrixText.match(/<!-- BEGIN TOOL IMPLEMENTATION MATRIX -->([\s\S]*?)<!-- END TOOL IMPLEMENTATION MATRIX -->/);
  if (!matrix) throw new Error("Matriz de ferramentas ausente.");
  const matrixTasks = new Map(data.tasks.filter((task) => /^H\d{2}$/.test(task.id) && task.document === matrixDocument).map((task) => [task.id, task]));
  if (!matrixTasks.size) throw new Error("Matriz de ferramentas sem tarefas associadas.");
  const seenMatrixIds = new Set();
  const matrixRows = matrix[1].split("\n").filter((line) => line.trim().startsWith("|")).map((line) => line.trim().slice(1, -1).split("|").map((cell) => cell.trim()));
  const [matrixHeader, ...matrixBody] = matrixRows;
  if (matrixHeader?.[0] !== "ID" || matrixHeader?.[3] !== "Status de implementação") throw new Error("Coluna obrigatória de status ausente na matriz de ferramentas.");
  for (const cells of matrixBody) {
    if (cells.every((cell) => /^:?-+:?$/.test(cell))) continue;
    const [id, , , status] = cells;
    if (cells.length !== matrixHeader.length || !matrixTasks.has(id) || seenMatrixIds.has(id)) throw new Error(`Linha inválida ou duplicada na matriz de ferramentas: ${id}.`);
    if (!["TODO", "IN PROGRESS", "DONE"].includes(status)) throw new Error(`Status de implementação obrigatório/inválido na matriz: ${id}.`);
    if (status !== matrixTasks.get(id).status) throw new Error(`Status da matriz diverge do tracker: ${id}.`);
    seenMatrixIds.add(id);
  }
  for (const id of matrixTasks.keys()) {
    if (!seenMatrixIds.has(id)) throw new Error(`Tarefa sem linha na matriz de ferramentas: ${id}.`);
  }
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
