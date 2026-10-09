"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { percentage, statusLabel, validate, filterTasks } = require("../progress/progress.js");
const data = JSON.parse(fs.readFileSync(path.join(__dirname, "../progress/tasks.json"), "utf8"));
const manifest = JSON.parse(fs.readFileSync(path.join(__dirname, "../docs/manifest.json"), "utf8"));
const routes = new Set(["home.md", "about.md", ...manifest.domains.flatMap((domain) => domain.files.map((file) => file.path))]);
const clone = () => structuredClone(data);

test("dados versionados têm documentos e evidências válidos", () => assert.equal(validate(data, routes), data));
test("percentual nunca anuncia 100% com checkpoint aberto e usa três dígitos", () => {
  const task = { status: "IN PROGRESS", checkpoints: Array.from({ length: 6 }, (_, index) => ({ completed: index < 2 })) };
  assert.equal(statusLabel(task), "IN PROGRESS (033%)");
  task.checkpoints.forEach((point) => { point.completed = false; });
  assert.equal(statusLabel(task), "IN PROGRESS (000%)");
  task.checkpoints = Array.from({ length: 1000 }, (_, index) => ({ completed: index < 999 }));
  assert.equal(percentage(task), 99);
});
test("TODO com trabalho concluído e DONE parcial são rejeitados", () => {
  for (const status of ["TODO", "DONE"]) {
    const changed = clone(); changed.tasks.find((task) => task.id === "U01").status = status;
    assert.throws(() => validate(changed, routes), /incompatível/);
  }
});
test("IN PROGRESS aceita zero e DONE exige todos os checkpoints com prova", () => {
  const changed = clone(); const task = changed.tasks.find((item) => item.id === "G01");
  task.status = "IN PROGRESS"; assert.doesNotThrow(() => validate(changed, routes));
  changed.evidence["geometry-fixture"] = { revision: "test fixture", document: task.document, summary: "Prova sintética para validar transições." };
  task.checkpoints.forEach((point) => { point.completed = true; point.evidence = "geometry-fixture"; });
  assert.throws(() => validate(changed, routes), /incompatível/);
  task.status = "DONE"; assert.doesNotThrow(() => validate(changed, routes));
  task.checkpoints[0].evidence = "prova-inexistente";
  assert.throws(() => validate(changed, routes), /evidência/);
  task.checkpoints[0].evidence = "ui-checkpoint";
  assert.throws(() => validate(changed, routes), /Evidência fora/);
});
test("bloqueia duplicação, quarto estado, links ausentes e datas inconsistentes", () => {
  for (const mutate of [
    (d) => { d.tasks[1].id = d.tasks[0].id; },
    (d) => { d.tasks[0].status = "BLOCKED"; },
    (d) => { d.tasks[0].document = "../fora.md"; },
    (d) => { d.tasks[0].documents.push("ausente.md"); },
    (d) => { d.tasks[0].updated = "2099-01-01"; },
    (d) => { d.tasks[0].checkpoints = []; }
  ]) { const changed = clone(); mutate(changed); assert.throws(() => validate(changed, routes)); }
});
test("busca ignora acentos e combina estado e etapa", () => {
  const tasks = [{ id: "X01", title: "Validação", description: "Teste", baseline: "", phase: "geometry", status: "TODO" }, { id: "X02", title: "Validação", description: "Teste", baseline: "", phase: "quality", status: "DONE" }];
  assert.deepEqual(filterTasks(tasks, { query: "validacao", status: "TODO", phase: "geometry" }).map((task) => task.id), ["X01"]);
  assert.equal(filterTasks(tasks, { query: "inexistente" }).length, 0);
  assert.equal(filterTasks(tasks, {}).length, 2);
});
