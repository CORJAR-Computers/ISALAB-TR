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
      sex: "F",
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
      schemaVersion: 11,
    }),
    get_dashboard_stats: () => {
      const list = samples.map(toListItem);
      const count = (st) => list.filter((s) => s.status === st).length;
      // Tendencia: últimos 7 días con el volumen de muestras recibidas.
      const weeklyVolume = [];
      for (let i = 6; i >= 0; i -= 1) {
        const d = new Date();
        d.setDate(d.getDate() - i);
        const key = d.toISOString().slice(0, 10);
        weeklyVolume.push({
          date: key,
          count: samples.filter((s) => s.receivedAt.slice(0, 10) === key).length,
        });
      }
      return {
        patientsTotal: patients.length,
        patientsActive: patients.length,
        samplesTotal: list.length,
        samplesInProgress: count("EN_PROCESO"),
        samplesFinished: count("FINALIZADA"),
        samplesCancelled: count("ANULADA"),
        abnormalResults: list.filter((s) => s.abnormalCount > 0).length,
        avgProcessingHours: null,
        abnormalRate: null,
        turnaroundBySampleType: [],
        weeklyVolume,
        topAnalytes: [],
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
    // Contadores por estado (v0.5.0): filas StatusCount + ABNORMAL/CRITICAL.
    count_samples: () => {
      const items = samples.map(toListItem);
      const rows = [];
      const seen = {};
      for (const s of items) {
        seen[s.status] = (seen[s.status] ?? 0) + 1;
      }
      for (const [status, count] of Object.entries(seen)) {
        rows.push({ status, count });
      }
      rows.push({
        status: "ABNORMAL",
        count: items.filter((i) => (i.abnormalCount ?? 0) > 0).length,
      });
      rows.push({
        status: "CRITICAL",
        count: items.filter((i) => (i.criticalCount ?? 0) > 0).length,
      });
      return rows;
    },
    list_panels: () => [],
    list_panel_analytes: () => [],
    list_qc_analyzer_status: () => [],
    list_analyzers: () => [
      {
        id: 1,
        code: "GENERAL",
        name: "Perfil GENERAL (lectura manual)",
        manufacturer: null,
        model: null,
        isActive: true,
        notes: null,
        rangeCount: 0,
      },
      {
        id: 2,
        code: "MB2800",
        name: "MINDRAY B2800",
        manufacturer: "Mindray",
        model: "B2800",
        isActive: true,
        notes: null,
        rangeCount: 0,
      },
    ],
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
        attachments: [],
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
    generate_sample_labels: (args) => {
      const ids = args.sampleIds ?? [];
      for (const id of ids) {
        if (!sampleById(id))
          throw { type: "NotFound", data: "Muestra no encontrada" };
      }
      return {
        path: `C:/mock/etiquetas-${ids.join("-")}.pdf`,
        fileName: `ISALAB_Etiquetas_${ids.join("-")}.pdf`,
        sampleCode: "M-2026-0001",
        generatedAt: new Date().toISOString().slice(0, 19).replace("T", " "),
      };
    },
    open_report_file: () => null,
    get_worklist: () => {
      const now = new Date();
      // Fecha LOCAL (igual que chrono::Local::now() en el backend).
      const p2 = (n) => String(n).padStart(2, "0");
      const today = `${now.getFullYear()}-${p2(now.getMonth() + 1)}-${p2(now.getDate())}`;
      const pending = samples.filter((s) =>
        ["RECIBIDA", "EN_PROCESO"].includes(s.status),
      );
      const elapsedMin = (s) => {
        const t = new Date(s.receivedAt.replace(" ", "T"));
        return Math.max(0, Math.floor((now.getTime() - t.getTime()) / 60000));
      };
      const toGroup = (list) => {
        const groups = [];
        for (const s of list) {
          const p = patientById(s.patientId);
          let g = groups.find((x) => x.sampleTypeId === s.sampleTypeId);
          if (!g) {
            g = {
              sampleTypeId: s.sampleTypeId,
              sampleTypeName: s.sampleTypeName,
              count: 0,
              maxElapsedMinutes: 0,
              samples: [],
            };
            groups.push(g);
          }
          g.count += 1;
          g.maxElapsedMinutes = Math.max(g.maxElapsedMinutes, elapsedMin(s));
          g.samples.push({
            id: s.id,
            code: s.code,
            patientId: s.patientId,
            patientName: p?.name ?? "?",
            ownerName: p?.ownerName ?? "?",
            speciesName: p?.speciesName ?? "?",
            sampleTypeId: s.sampleTypeId,
            sampleTypeName: s.sampleTypeName,
            status: s.status,
            receivedAt: s.receivedAt,
            elapsedMinutes: elapsedMin(s),
            resultCount: s.results.length,
            abnormalCount: s.results.filter(
              (r) => r.status === "ALTO" || r.status === "BAJO",
            ).length,
          });
        }
        return groups.sort((a, b) => b.maxElapsedMinutes - a.maxElapsedMinutes);
      };
      const todayList = pending.filter((s) => s.receivedAt.slice(0, 10) === today);
      const overdueList = pending.filter((s) => s.receivedAt.slice(0, 10) !== today);
      return {
        date: today,
        totalPending: pending.length,
        today: toGroup(todayList),
        overdue: toGroup(overdueList),
      };
    },
    global_search: (args) => {
      const q = (args.query ?? "").trim().toLowerCase();
      if (!q) return [];
      const results = [];
      for (const p of patients) {
        if (
          p.name.toLowerCase().includes(q) ||
          p.code.toLowerCase().includes(q) ||
          p.ownerName.toLowerCase().includes(q)
        ) {
          results.push({
            kind: "patient",
            id: p.id,
            title: p.name,
            subtitle: `${p.speciesName} · ${p.ownerName}`,
            code: p.code,
          });
        }
      }
      for (const s of samples) {
        const p = patientById(s.patientId);
        if (
          s.code.toLowerCase().includes(q) ||
          (p?.name ?? "").toLowerCase().includes(q)
        ) {
          results.push({
            kind: "sample",
            id: s.id,
            title: p?.name ?? "?",
            subtitle: `${s.sampleTypeName} · ${s.status}`, // eslint-disable-line
            code: s.code,
          });
        }
      }
      // Coincidencias por prefijo primero (igual que el backend).
      return results.sort((a, b) => {
        const ap =
          a.title.toLowerCase().startsWith(q) ||
          (a.code ?? "").toLowerCase().startsWith(q)
            ? 0
            : 1;
        const bp =
          b.title.toLowerCase().startsWith(q) ||
          (b.code ?? "").toLowerCase().startsWith(q)
            ? 0
            : 1;
        return ap - bp;
      });
    },
    get_clinic_settings: () => ({
      clinicName: "Clínica Veterinaria Central",
      clinicNit: "900000000-0",
      address: "Calle 12 # 34-56",
      phone: "3001234567",
      city: "Bogotá D.C.",
      logoPath: null,
      taxRate: 19,
      currency: "COP",
      signatureMode: "GRAPHIC",
      vetName: "Dra. Ana Pérez",
      vetLicense: "MVZ 12345",
      groqApiKey: null,
      pkcs12Path: null,
      pkcs12Password: null,
    }),
    save_clinic_settings: (args) => args.input,
    import_clinic_logo: (args) => args.sourcePath,
    list_secondary_logos: () => [
      {
        id: 1,
        name: "Logo Falso",
        logoPath: "C:/logos/falso.png",
        createdAt: "2026-08-01 10:00:00",
      },
      {
        id: 2,
        name: "Proyecto X",
        logoPath: "C:/logos/proyecto-x.png",
        createdAt: "2026-08-02 11:00:00",
      },
    ],
    import_secondary_logo: (args) => ({
      id: 3,
      name: args.input.name,
      logoPath: args.input.sourcePath,
      createdAt: "2026-08-03 12:00:00",
    }),
    delete_secondary_logo: () => null,
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
    // La UI usa convertFileSrc (p. ej. para previsualizar logos). En el
    // navegador no existe el protocolo asset:// de Tauri, así que se devuelve
    // un PNG transparente 1x1 para que el <img> cargue sin errores.
    convertFileSrc: () =>
      "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==",
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
