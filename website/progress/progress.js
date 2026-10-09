(function (root) {
  "use strict";

  const STATUSES = ["TODO", "IN PROGRESS", "DONE"];
  const normalize = (value) => String(value).normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  const escape = (value) => String(value).replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[char]));

  function percentage(task) {
    return Math.floor(100 * task.checkpoints.filter((item) => item.completed).length / task.checkpoints.length);
  }

  function statusLabel(task) {
    return task.status === "IN PROGRESS" ? `IN PROGRESS (${String(percentage(task)).padStart(3, "0")}%)` : task.status;
  }

  function validate(data, knownDocuments) {
    if (data.schemaVersion !== 1 || data.branch !== "refactor/architecture-foundation" || !/^[a-f0-9]{40}$/.test(data.baselineSha)) {
      throw new Error("Versão, branch ou baseline inválida no acompanhamento.");
    }
    if (!Array.isArray(data.phases) || !data.phases.length || !Array.isArray(data.tasks) || !data.tasks.length || !data.evidence) {
      throw new Error("Fases, tarefas ou evidências ausentes.");
    }
    const phases = new Set();
    for (const phase of data.phases) {
      if (!phase.id || !phase.title || phases.has(phase.id)) throw new Error("Fase inválida ou duplicada.");
      phases.add(phase.id);
    }
    const validDate = (value) => /^\d{4}-\d{2}-\d{2}$/.test(value) && new Date(value).toISOString().slice(0, 10) === value;
    if (!validDate(data.updated)) throw new Error("Data do acompanhamento inválida.");
    for (const proof of Object.values(data.evidence)) {
      if (!proof.revision || !proof.summary || !knownDocuments.has(proof.document)) throw new Error("Evidência sem revisão, resumo ou documento registrado.");
    }
    const ids = new Set();
    for (const task of data.tasks) {
      if (!/^[A-Z]\d{2}$/.test(task.id) || ids.has(task.id)) throw new Error("ID de tarefa inválido ou duplicado.");
      ids.add(task.id);
      if (!Array.isArray(task.documents) || task.documents[0] !== task.document || new Set(task.documents).size !== task.documents.length || task.documents.some((path) => !knownDocuments.has(path))) {
        throw new Error(`Leituras complementares inválidas: ${task.id}.`);
      }
      if (!phases.has(task.phase) || !task.title || !task.description || !knownDocuments.has(task.document) || !validDate(task.updated) || task.updated > data.updated) {
        throw new Error(`Metadados ou documento inválido: ${task.id}.`);
      }
      if (!STATUSES.includes(task.status) || !Array.isArray(task.checkpoints) || !task.checkpoints.length) throw new Error(`Estado ou checkpoints inválidos: ${task.id}.`);
      for (const point of task.checkpoints) {
        if (!point.title || typeof point.completed !== "boolean" || (point.completed && !Object.hasOwn(data.evidence, point.evidence))) {
          throw new Error(`Checkpoint sem critério ou evidência: ${task.id}.`);
        }
        if (point.completed && !task.documents.includes(data.evidence[point.evidence].document)) {
          throw new Error(`Evidência fora dos documentos da tarefa: ${task.id}.`);
        }
      }
      const completed = task.checkpoints.filter((point) => point.completed).length;
      if ((task.status === "TODO" && completed !== 0) || (task.status === "DONE" && completed !== task.checkpoints.length) || (task.status === "IN PROGRESS" && completed === task.checkpoints.length)) {
        throw new Error(`Estado incompatível com checkpoints: ${task.id}.`);
      }
    }
    return data;
  }

  function filterTasks(tasks, filters) {
    const query = normalize(filters.query || "").trim();
    return tasks.filter((task) => (!filters.status || task.status === filters.status) && (!filters.phase || task.phase === filters.phase) && normalize(`${task.id} ${task.title} ${task.description} ${task.baseline || ""}`).includes(query));
  }

  async function mount(container, manifest, isCurrent, taskId) {
    const response = await fetch("./progress/tasks.json", { cache: "no-store" });
    if (!response.ok) throw new Error(`Falha ao carregar o acompanhamento (${response.status}).`);
    const documents = new Map(manifest.domains.flatMap((domain) => domain.files.map((file) => [file.path, file.title])));
    const data = validate(await response.json(), new Set([...documents.keys(), "home.md", "about.md"]));
    if (!isCurrent()) return;
    const docLink = (path, title) => `<a href="#/docs/${escape(path)}">${escape(title || documents.get(path))}</a>`;
    const counts = Object.fromEntries(STATUSES.map((status) => [status, data.tasks.filter((task) => task.status === status).length]));
    container.innerHTML = `<article class="progress-page" aria-labelledby="progress-title">
      <div class="doc-context"><i class="ph ph-check-square" aria-hidden="true"></i><span>Reimplementação</span></div>
      <header class="progress-header"><span class="eyebrow">Caderno vivo · ${escape(data.branch)}</span><h1 id="progress-title">Acompanhamento da reimplementação</h1>
        <p>Da decisão à entrega. Cada processo tem checkpoints verificáveis e um documento canônico.</p>
        <p class="progress-meta">Atualizado em ${escape(data.updated.split("-").reverse().join("/"))} · ${data.tasks.length} processos · ${docLink("17-reimplementation/index.md", "Como atualizar esta tabela")}</p>
      </header>
      <dl class="progress-counts"><div><dt>TODO</dt><dd>${counts.TODO}</dd></div><div><dt>IN PROGRESS</dt><dd>${counts["IN PROGRESS"]}</dd></div><div><dt>DONE</dt><dd>${counts.DONE}</dd></div></dl>
      <form class="progress-filters" role="search" aria-label="Filtrar processos">
        <div><label for="task-query">Buscar processo</label><input id="task-query" type="search" placeholder="Nome, ID ou requisito" autocomplete="off"></div>
        <div><label for="task-status">Status</label><select id="task-status"><option value="">Todos os estados</option>${STATUSES.map((status) => `<option>${status}</option>`).join("")}</select></div>
        <div><label for="task-phase">Etapa</label><select id="task-phase"><option value="">Todas as etapas</option>${data.phases.map((phase) => `<option value="${escape(phase.id)}">${escape(phase.title)}</option>`).join("")}</select></div>
        <button type="reset" class="progress-reset">Limpar filtros</button>
      </form>
      <p class="progress-note">Percentuais contam checkpoints concluídos; não estimam esforço ou tempo. Código legado existente deve ser preservado, mas não comprova a migração.</p>
      <p id="task-result-count" class="progress-meta" role="status" aria-live="polite" aria-atomic="true"></p>
      <div id="task-results"></div>
    </article>`;
    const results = container.querySelector("#task-results");
    const query = container.querySelector("#task-query");
    const status = container.querySelector("#task-status");
    const phase = container.querySelector("#task-phase");
    const row = (task) => {
      const completed = task.checkpoints.filter((point) => point.completed).length;
      const stateClass = task.status === "DONE" ? "done" : task.status === "IN PROGRESS" ? "in-progress" : "todo";
      return `<tr id="task-${task.id}"><th scope="row"><a class="task-permalink" href="#/progress/${task.id}" aria-label="Link para o processo ${task.id}">${task.id}</a></th>
        <td class="task-description"><strong>${docLink(task.document, task.title)}</strong><p>${escape(task.description)}</p>
          <details><summary>Checkpoints e evidências · ${completed}/${task.checkpoints.length}</summary><p class="task-baseline">${escape(task.baseline || "")}</p>
          <ul class="task-checkpoints">${task.checkpoints.map((point) => `<li><span class="checkpoint-state ${point.completed ? "complete" : ""}">${point.completed ? "Concluído" : "Pendente"}</span> ${escape(point.title)}${point.completed ? `<small>${docLink(data.evidence[point.evidence].document, "Evidência")} · ${escape(data.evidence[point.evidence].summary)}<br>Revisão: ${escape(data.evidence[point.evidence].revision)}</small>` : ""}</li>`).join("")}</ul>
          ${task.documents.length > 1 ? `<p class="task-baseline">Leituras complementares: ${task.documents.slice(1).map((path) => docLink(path, documents.get(path) || path)).join(" · ")}</p>` : ""}</details></td>
        <td><span class="task-status ${stateClass}">${statusLabel(task)}</span>${task.status === "IN PROGRESS" ? `<progress value="${completed}" max="${task.checkpoints.length}" aria-label="Checkpoints concluídos de ${task.id}"></progress>` : ""}<small class="task-date">${escape(task.updated.split("-").reverse().join("/"))}</small></td>
        <td class="task-document">${docLink(task.document)}</td></tr>`;
    };
    function render() {
      const visible = filterTasks(data.tasks, { query: query.value, status: status.value, phase: phase.value });
      container.querySelector("#task-result-count").textContent = `${visible.length} de ${data.tasks.length} processos`;
      results.innerHTML = visible.length ? data.phases.map((item) => {
        const tasks = visible.filter((task) => task.phase === item.id);
        if (!tasks.length) return "";
        return `<section class="progress-phase" aria-labelledby="phase-${escape(item.id)}"><h2 id="phase-${escape(item.id)}">${escape(item.title)} <span>${tasks.length}</span></h2><p>${escape(item.description)}</p>
          <div class="progress-table-scroll" role="region" aria-label="Processos: ${escape(item.title)}" tabindex="0"><table><caption class="sr-only">Processos da etapa ${escape(item.title)}</caption><thead><tr><th scope="col">ID</th><th scope="col">Processo</th><th scope="col">Status</th><th scope="col">Documento</th></tr></thead><tbody>${tasks.map(row).join("")}</tbody></table></div></section>`;
      }).join("") : '<div class="progress-empty">Nenhum processo corresponde aos filtros. Use “Limpar filtros” para ver a tabela completa.</div>';
    }
    container.querySelector("form").addEventListener("submit", (event) => event.preventDefault());
    query.addEventListener("input", render);
    status.addEventListener("change", render);
    phase.addEventListener("change", render);
    container.querySelector("form").addEventListener("reset", () => { query.value = ""; status.value = ""; phase.value = ""; render(); });
    render();
    if (taskId) {
      const target = container.querySelector(`#task-${taskId}`);
      if (target) { target.classList.add("task-highlight"); target.scrollIntoView({ block: "center", behavior: "auto" }); }
    } else { window.scrollTo({ top: 0, behavior: "auto" }); }
    container.focus({ preventScroll: true });
  }

  const api = { percentage, statusLabel, validate, filterTasks, mount };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.PetuniaProgress = api;
})(typeof window !== "undefined" ? window : this);
