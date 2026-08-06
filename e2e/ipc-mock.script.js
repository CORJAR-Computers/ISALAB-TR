// Mock del runtime de Tauri para los tests E2E (Playwright).
//
// Se inyecta con page.addInitScript ANTES de que cargue la app: define
// window.__TAURI_INTERNALS__ (que es lo que usa @tauri-apps/api/core#invoke)
// y responde los comandos del flujo smoke con un estado en memoria.
// Los comandos fuera del flujo devuelven null sin romper la app.
//
// Script en JS plano de navegador (sin imports): se ejecuta en la página.
(() => {
  if (window.__TAURI_INTERNALS__) return;

  // ---------- Estado en memoria ----------
  const patients = [
    {
      id: 1,
      code: "P-0001",
      ownerId: 10,
      speciesId: 1,
      breedId: 1,
      name: "Rocky",
      sex: "M",
      birthDate: "2022-03-15",
      neutered: true,
      color: "Dorado",
      microchip: "1234567890",
      active: true,
      notes: null,
      preferredLogoId: null,
      speciesName: "Canino",
      breedName: "Labrador",
      ownerName: "Juan Pérez",
      ownerPhone: "3001234567",
      ageMonths: 52,
    },
    {
      id: 2,
      code: "P-0002",
      ownerId: 10,
      speciesId: 1,
      breedId: 2,
      name: "Luna",
      sex: "H",
      birthDate: "2023-07-01",
      neutered: true,
      color: "Negro",
      microchip: null,
      active: true,
      notes: null,
      preferredLogoId: null,
      speciesName: "Canino",
      breedName: "Criollo",
      ownerName: "Juan Pérez",
      ownerPhone: "3001234567",
      ageMonths: 37,
    },
  ];

  const sampleTypes = [
    { id: 1, code: "BLOOD", name: "Sangre" },
    { id: 2, code: "SERUM", name: "Suero" },
    { id: 3, code: "URINE", name: "Orina" },
  ];

  const analytes = [
    { id: 1, code: "GLU", name: "Glucosa", unit: "mg/dL", method: null },
    { id: 2, code: "HCT", name: "Hematocrito", unit: "%", method: null },
    { id: 3, code: "UREA", name: "Urea", unit: "mg/dL", method: null },
  ];

  let samples = []; // { ...Sample, results: LabResult[] }
  let nextSampleId = 1;
  let nextResultId = 1;

  const pad4 = (n) => String(n).padStart(4, "0");
  const sampleCode = () => `M-2026-${pad4(nextSampleId)}`;
  const patientById = (id) => patients.find((p) => p.id === id);
  const sampleTypeById = (id) => sampleTypes.find((t) => t.id === id);
  const sampleById = (id) => samples.find((s) => s.id === id);

  const toListItem = (s) => {
    const p = patientById(s.patientId);
    return {
      id: s.id,
      code: s.code,
      patientId: s.patientId,
      patientName: p?.name ?? "?",
      ownerName: p?.ownerName ?? "?",
      speciesName: p?.speciesName ?? "?",
      sampleTypeId: s.sampleTypeId,
      sampleTypeName: s.sampleTypeName,
      receivedAt: s.receivedAt,
      status: s.status,
      collectedBy: s.collectedBy,
      notes: s.notes,
      resultCount: s.results.length,
      abnormalCount: s.results.filter(
        (r) => r.status === "ALTO" || r.status === "BAJO",
      ).length,
    };
  };

  // Estado clínico simplificado (el real lo calcula SP_VALIDATE_ANALYTICAL_RESULT).
  const resultMeta = (analyteId, value) => {
    if (analyteId === 1)
      return value > 126
        ? { status: "ALTO", refMin: 70, refMax: 126 }
        : { status: "NORMAL", refMin: 70, refMax: 126 };
    if (analyteId === 2)
      return { status: "NORMAL", refMin: 37, refMax: 55 };
    return { status: "NORMAL", refMin: null, refMax: null };
  };

  // ---------- Handlers por comando Tauri ----------
  const handlers = {
    get_session: () => null,
    login: (args) => {
      const { username, password } = args.input ?? {};
      if (!username || !password)
        throw { type: "Validation", data: "Credenciales inválidas" };
      return {
        id: 1,
        username,
        fullName: "Administrador",
        role: "ADMIN",
        mustChangePassword: false,
      };
    },
    logout: () => null,
    db_health: () => ({
      ok: true,
      message: "ok",
      dbPath: "C:/mock/isalab.fdb",
      fbclientFound: true,
      fbclientPath: "fbclient.dll",
      schemaVersion: 9,
    }),
    get_dashboard_stats: () => {
      const list = samples.map(toListItem);
      const count = (st) => list.filter((s) => s.status === st).length;
      return {
        patientsTotal: patients.length,
        patientsActive: patients.length,
        samplesTotal: list.length,
        samplesInProgress: count("EN_PROCESO"),
        samplesFinished: count("FINALIZADA"),
        samplesCancelled: count("ANULADA"),
        abnormalResults: list.filter((s) => s.abnormalCount > 0).length,
        consultationsPending: 0,
        surgeriesProgrammed: 0,
        vaccinesDue: 0,
        invoicesUnpaid: 0,
        revenueTotal: 0,
        upcomingConsultations: [],
        upcomingSurgeries: [],
        upcomingVaccines: [],
        recentSamples: list.slice(-5).reverse(),
      };
    },
    list_sample_types: () => sampleTypes,
    list_analytes: () => analytes,
    list_patients: (args) => {
      const q = (args.search ?? "").toLowerCase();
      if (!q) return patients;
      return patients.filter(
        (p) =>
          p.name.toLowerCase().includes(q) ||
          p.ownerName.toLowerCase().includes(q),
      );
    },
    get_patient: (args) => patientById(args.id) ?? null,
    list_samples: (args) => {
      let list = samples.map(toListItem).reverse(); // más reciente primero
      if (args.status)
        list = list.filter((s) => s.status === args.status);
      if (args.search) {
        const q = args.search.toLowerCase();
        list = list.filter(
          (s) =>
            s.code.toLowerCase().includes(q) ||
            s.patientName.toLowerCase().includes(q) ||
            s.ownerName.toLowerCase().includes(q),
        );
      }
      return list;
    },
    get_sample: (args) => sampleById(args.id) ?? null,
    create_sample: (args) => {
      const input = args.input;
      const p = patientById(input.patientId);
      const t = sampleTypeById(input.sampleTypeId);
      if (!p || !t) throw { type: "Validation", data: "Paciente o tipo inválido" };
      const sample = {
        id: nextSampleId,
        code: sampleCode(),
        patientId: input.patientId,
        sampleTypeId: input.sampleTypeId,
        sampleTypeName: t.name,
        receivedAt: input.receivedAt,
        status: "RECIBIDA",
        collectedBy: input.collectedBy ?? null,
        notes: input.notes ?? null,
        results: [],
      };
      nextSampleId += 1;
      samples.push(sample);
      return sample;
    },
    register_lab_result: (args) => {
      const s = sampleById(args.input.sampleId);
      if (!s) throw { type: "NotFound", data: "Muestra no encontrada" };
      const a = analytes.find((x) => x.id === args.input.analyteId);
      if (!a) throw { type: "Validation", data: "Analito inválido" };
      const { status, refMin, refMax } = resultMeta(
        args.input.analyteId,
        args.input.value,
      );
      const result = {
        id: nextResultId,
        sampleId: s.id,
        analyteId: a.id,
        analyteName: a.name,
        unit: a.unit,
        value: args.input.value,
        status,
        refMin,
        refMax,
        analyzedAt: new Date().toISOString().slice(0, 19).replace("T", " "),
      };
      nextResultId += 1;
      s.results.push(result);
      // Como el SP real: al cargar un resultado la muestra pasa a EN_PROCESO.
      if (s.status === "RECIBIDA") s.status = "EN_PROCESO";
      return result;
    },
    set_sample_status: (args) => {
      const s = sampleById(args.id);
      if (!s) throw { type: "NotFound", data: "Muestra no encontrada" };
      s.status = args.status;
      return { ...s, results: [...s.results] };
    },
    // Listeners/emits del runtime Tauri (Firebird events, app-ready, …)
    "plugin:event|listen": () => () => {},
    "plugin:event|unlisten": () => null,
    "plugin:event|emit": () => null,
  };

  const invoke = (cmd, args = {}) => {
    const fn = handlers[cmd];
    if (!fn) {
      console.warn("[e2e-ipc-mock] Comando sin implementar:", cmd, args);
      return Promise.resolve(null);
    }
    try {
      return Promise.resolve(fn(args));
    } catch (err) {
      return Promise.reject(err);
    }
  };

  let callbackId = 0;
  window.__TAURI_INTERNALS__ = {
    invoke,
    // getCurrentWindow() lee esta metadata (etiqueta de la ventana).
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { label: "main" },
      windows: [{ label: "main" }],
      webviews: [{ label: "main" }],
    },
    transformCallback: (cb) => {
      callbackId += 1;
      const id = `_${callbackId}`;
      window[id] = cb;
      return id;
    },
  };
})();
