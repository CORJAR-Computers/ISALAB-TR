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

  // Catálogo de analitos: los 3 históricos + los que siembra la migración
  // 0021 desde Valores_Referencia_Veterinarios.md (ALP/ALT/CREA).
  const analytes = [
    { id: 1, code: "GLU", name: "Glucosa", unit: "mg/dL", method: null },
    { id: 2, code: "HCT", name: "Hematocrito", unit: "%", method: null },
    { id: 3, code: "UREA", name: "Urea", unit: "mg/dL", method: null },
    { id: 4, code: "ALP", name: "Fosfatasa alcalina (ALP)", unit: "U/L", method: "Cinético IFCC" },
    { id: 5, code: "ALT", name: "Alanina aminotransferasa (ALT)", unit: "U/L", method: "Cinético IFCC" },
    { id: 6, code: "CREA", name: "Creatinina", unit: "mg/dL", method: null },
  ];
  let nextAnalyteId = 7;

  // Panel por defecto ("Química básica") con los 3 analitos del catálogo.
  // La grilla de resultados del detalle usa list_panels/list_panel_analytes.
  const panels = [
    { id: 1, name: "Química básica", sampleTypeId: null, sampleTypeName: null, sortOrder: 0, isActive: true, notes: null, analyteCount: 3 },
  ];
  const panelAnalytes = [
    { analyteId: 1, analyteName: "Glucosa", unit: "mg/dL", seq: 1 },
    { analyteId: 2, analyteName: "Hematocrito", unit: "%", seq: 2 },
    { analyteId: 3, analyteName: "Urea", unit: "mg/dL", seq: 3 },
  ];

  // Rangos del perfil GENERAL (id 1): los históricos + los sembrados de la
  // migración 0021 desde Valores_Referencia_Veterinarios.md (mismos valores
  // por especie: canino/felino/equino). La grilla del detalle los usa para
  // evaluar Normal/Alto/Bajo.
  const referenceRanges = [
    { id: 1, analyzerId: 1, analyzerName: "GENERAL", analyteId: 1, analyteName: "Glucosa", unit: "mg/dL", speciesId: 1, speciesName: "Canino", sex: null, ageMinMonths: 0, ageMaxMonths: 0, minValue: 70, maxValue: 126, criticalMin: 40, criticalMax: 300, notes: null },
    { id: 2, analyzerId: 1, analyzerName: "GENERAL", analyteId: 2, analyteName: "Hematocrito", unit: "%", speciesId: 1, speciesName: "Canino", sex: null, ageMinMonths: 0, ageMaxMonths: 0, minValue: 37, maxValue: 55, criticalMin: 20, criticalMax: 65, notes: null },
    { id: 3, analyzerId: 1, analyzerName: "GENERAL", analyteId: 1, analyteName: "Glucosa", unit: "mg/dL", speciesId: 2, speciesName: "Felino", sex: null, ageMinMonths: 0, ageMaxMonths: 0, minValue: 74, maxValue: 159, criticalMin: null, criticalMax: null, notes: null },
    { id: 4, analyzerId: 1, analyzerName: "GENERAL", analyteId: 1, analyteName: "Glucosa", unit: "mg/dL", speciesId: 3, speciesName: "Equino", sex: null, ageMinMonths: 0, ageMaxMonths: 0, minValue: 62, maxValue: 117, criticalMin: null, criticalMax: null, notes: null },
    { id: 5, analyzerId: 1, analyzerName: "GENERAL", analyteId: 4, analyteName: "Fosfatasa alcalina (ALP)", unit: "U/L", speciesId: 1, speciesName: "Canino", sex: null, ageMinMonths: 0, ageMaxMonths: 0, minValue: 7, maxValue: 115, criticalMin: null, criticalMax: null, notes: null },
    { id: 6, analyzerId: 1, analyzerName: "GENERAL", analyteId: 4, analyteName: "Fosfatasa alcalina (ALP)", unit: "U/L", speciesId: 2, speciesName: "Felino", sex: null, ageMinMonths: 0, ageMaxMonths: 0, minValue: 11, maxValue: 49, criticalMin: null, criticalMax: null, notes: null },
    { id: 7, analyzerId: 1, analyzerName: "GENERAL", analyteId: 4, analyteName: "Fosfatasa alcalina (ALP)", unit: "U/L", speciesId: 3, speciesName: "Equino", sex: null, ageMinMonths: 0, ageMaxMonths: 0, minValue: 88, maxValue: 261, criticalMin: null, criticalMax: null, notes: null },
  ];
  let nextRangeId = 8;

  let samples = []; // { ...Sample, results: LabResult[] }
  let nextSampleId = 1;
  let nextResultId = 1;

  // Historial y notificaciones sembrados para la primera muestra del flujo
  // (la que crea el smoke test recibe id 1). Ejercitan los EventRow y
  // NotificationRow de los diálogos apilados del detalle.
  const sampleEvents = [
    { id: 1, sampleId: 1, eventType: "REJECTED", username: "admin", reason: "Tubo sin etiquetar", createdAt: "2026-09-07 10:15:00" },
    { id: 2, sampleId: 1, eventType: "REOPENED", username: "admin", reason: null, createdAt: "2026-09-07 11:40:00" },
  ];
  const notificationsLog = [
    { id: 1, resultId: null, sampleId: 1, channel: "EMAIL", recipientName: "Juan Pérez", recipientAddress: "juan.perez@example.com", status: "SENT", sentAt: "2026-09-07 12:05:00", ackedAt: null, ackedBy: null, note: null, createdAt: "2026-09-07 12:05:00" },
    { id: 2, resultId: null, sampleId: 1, channel: "MANUAL", recipientName: null, recipientAddress: null, status: "ACKNOWLEDGED", sentAt: null, ackedAt: "2026-09-07 12:20:00", ackedBy: "Dra. Ana Pérez", note: null, createdAt: "2026-09-07 12:20:00" },
  ];

  // Especies del catálogo (ids 1/2/3 = canino/felino/equino, como el seed).
  const species = [
    { id: 1, code: "CAN", name: "Canino" },
    { id: 2, code: "FEL", name: "Felino" },
    { id: 3, code: "EQU", name: "Equino" },
  ];

  // Equipos del catálogo (id 1 = perfil GENERAL). rangeCount se calcula de
  // referenceRanges para reflejar los rangos sembrados en la UI.
  const analyzers = [
    { id: 1, code: "GENERAL", name: "Perfil GENERAL (lectura manual)", manufacturer: null, model: null, isActive: true, notes: null },
    { id: 2, code: "MB2800", name: "MINDRAY B2800", manufacturer: "Mindray", model: "B2800", isActive: true, notes: null },
  ];

  const pad4 = (n) => String(n).padStart(4, "0");
  const sampleCode = () => `M-2026-${pad4(nextSampleId)}`;
  const patientById = (id) => patients.find((p) => p.id === id);
  const sampleTypeById = (id) => sampleTypes.find((t) => t.id === id);
  const sampleById = (id) => samples.find((s) => s.id === id);
  const analyzerById = (id) => analyzers.find((a) => a.id === id);
  const speciesById = (id) => species.find((s) => s.id === id);

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
    list_panels: () => panels,
    // Un solo panel (id 1) con los tres analitos del catálogo.
    list_panel_analytes: (args) => (args.panelId === 1 ? panelAnalytes : []),
    list_qc_analyzer_status: () => [],
    list_reference_ranges: (args) =>
      args.analyzerId != null
        ? referenceRanges.filter((r) => r.analyzerId === args.analyzerId)
        : referenceRanges,
    create_analyte: (args) => {
      const { code, name, unit, method, description } = args.input ?? {};
      if (!code || !name)
        throw { type: "Validation", data: "Código y nombre son obligatorios" };
      const normalized = code.toUpperCase();
      if (analytes.some((a) => a.code === normalized))
        throw { type: "Conflict", data: `Ya existe el analito ${normalized}` };
      const analyte = {
        id: nextAnalyteId,
        code: normalized,
        name,
        unit: unit ?? null,
        method: method ?? null,
      };
      nextAnalyteId += 1;
      analytes.push(analyte);
      return analyte;
    },
    create_reference_range: (args) => {
      const input = args.input ?? {};
      const a = analytes.find((x) => x.id === input.analyteId);
      const s = speciesById(input.speciesId);
      if (!a || !s) throw { type: "Validation", data: "Analito o especie inválidos" };
      const range = {
        id: nextRangeId,
        analyzerId: input.analyzerId,
        analyzerName: analyzerById(input.analyzerId)?.name ?? "?",
        analyteId: a.id,
        analyteName: a.name,
        unit: a.unit,
        speciesId: s.id,
        speciesName: s.name,
        sex: input.sex ?? null,
        ageMinMonths: input.ageMinMonths,
        ageMaxMonths: input.ageMaxMonths,
        minValue: input.minValue,
        maxValue: input.maxValue,
        criticalMin: input.criticalMin ?? null,
        criticalMax: input.criticalMax ?? null,
        notes: input.notes ?? null,
      };
      nextRangeId += 1;
      referenceRanges.push(range);
      return range;
    },
    update_reference_range: (args) => {
      const r = referenceRanges.find((x) => x.id === args.id);
      if (!r) throw { type: "NotFound", data: "Rango no encontrado" };
      const input = args.input ?? {};
      const a = analytes.find((x) => x.id === input.analyteId);
      const s = speciesById(input.speciesId);
      if (!a || !s) throw { type: "Validation", data: "Analito o especie inválidos" };
      r.analyteId = input.analyteId;
      r.analyteName = a.name;
      r.unit = a.unit;
      r.speciesId = input.speciesId;
      r.speciesName = s.name;
      r.sex = input.sex ?? null;
      r.ageMinMonths = input.ageMinMonths;
      r.ageMaxMonths = input.ageMaxMonths;
      r.minValue = input.minValue;
      r.maxValue = input.maxValue;
      r.criticalMin = input.criticalMin ?? null;
      r.criticalMax = input.criticalMax ?? null;
      r.notes = input.notes ?? null;
      return { ...r };
    },
    list_analyzers: () =>
      analyzers.map((a) => ({
        ...a,
        rangeCount: referenceRanges.filter((r) => r.analyzerId === a.id).length,
      })),
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
    list_sample_events: (args) =>
      sampleEvents.filter((e) => e.sampleId === args.sampleId),
    list_sample_notifications: (args) =>
      notificationsLog.filter((n) => n.sampleId === args.sampleId),
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
        analyzerId: input.analyzerId ?? null,
        analyzerName: input.analyzerId != null ? analyzerById(input.analyzerId)?.name ?? null : null,
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
        deltaVariation: null,
        isCritical: false,
        attachments: [],
      };
      nextResultId += 1;
      s.results.push(result);
      // Como el SP real: al cargar un resultado la muestra pasa a EN_PROCESO.
      if (s.status === "RECIBIDA") s.status = "EN_PROCESO";
      return result;
    },
    // Carga por lotes (grilla del detalle): reemplaza valores y elimina vaciados.
    register_lab_results: (args) => {
      const s = sampleById(args.input.sampleId);
      if (!s) throw { type: "NotFound", data: "Muestra no encontrada" };
      const saved = [];
      for (const item of args.input.results ?? []) {
        const a = analytes.find((x) => x.id === item.analyteId);
        if (!a) throw { type: "Validation", data: `Analito inválido: ${item.analyteId}` };
        // Reemplaza el valor previo del analito (upsert).
        s.results = s.results.filter((r) => r.analyteId !== a.id);
        const { status, refMin, refMax } = resultMeta(a.id, item.value);
        const result = {
          id: nextResultId,
          sampleId: s.id,
          analyteId: a.id,
          analyteName: a.name,
          unit: a.unit,
          value: item.value,
          status,
          refMin,
          refMax,
          analyzedAt: new Date().toISOString().slice(0, 19).replace("T", " "),
          deltaVariation: null,
          isCritical: status === "CRITICO_BAJO" || status === "CRITICO_ALTO",
          attachments: [],
        };
        nextResultId += 1;
        s.results.push(result);
        saved.push(result);
      }
      if (saved.length > 0 && s.status === "RECIBIDA") s.status = "EN_PROCESO";
      return saved;
    },
    delete_lab_result: (args) => {
      const s = sampleById(args.sampleId);
      if (!s) throw { type: "NotFound", data: "Muestra no encontrada" };
      s.results = s.results.filter((r) => r.analyteId !== args.analyteId);
      return null;
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
    // ----- Catálogos y páginas del resto de la app (estados vacíos) -------
    // Necesarios para el test de viewport 1366x768, que navega por TODAS las
    // vistas: cada página debe montar sin datos y sin excepciones.
    list_species: () => species,
    list_breeds: () => [
      { id: 1, speciesId: 1, name: "Labrador" },
      { id: 2, speciesId: 1, name: "Criollo" },
    ],
    list_owners: () => [
      {
        id: 10,
        documentType: "CC",
        documentNumber: "1020304050",
        fullName: "Juan Pérez",
        phone: "3001234567",
        email: null,
        address: null,
        city: "Bogotá",
      },
    ],
    list_users: () => [
      {
        id: 1,
        username: "admin",
        fullName: "Administrador",
        role: "ADMIN",
        active: true,
        mustChangePassword: false,
        createdAt: "2026-08-01 08:00:00",
      },
      {
        id: 2,
        username: "mv.perez",
        fullName: "Dra. Ana Pérez",
        role: "VETERINARIO",
        active: true,
        mustChangePassword: false,
        createdAt: "2026-08-02 09:30:00",
      },
    ],
    list_audit_log: () => [
      {
        id: 1,
        userId: 1,
        username: "admin",
        action: "LOGIN",
        details: null,
        createdAt: "2026-09-08 08:00:00",
      },
      {
        id: 2,
        userId: 1,
        username: "admin",
        action: "SETTINGS_CHANGED",
        details: "Datos de la clínica actualizados",
        createdAt: "2026-09-08 08:15:00",
      },
    ],
    list_consultations: () => [],
    count_consultations: () => [],
    list_surgeries: () => [],
    count_surgeries: () => [],
    list_vaccines: () => [],
    list_invoices: () => [],
    count_invoices: () => [],
    list_lab_orders: () => [],
    count_lab_orders: () => [],
    list_qc_materials: () => [],
    list_qc_runs: () => [],
    list_qc_targets: () => [],
    list_qc_analyzer_status: () => [],
    get_qc_chart: () => null,
    list_reports: () => [],
    list_analyzer_sources: () => [],
    list_analyzer_import_jobs: () => [],
    list_failed_analyzer_imports: () => [],
    poll_analyzer_source: () => null,
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
  // La API de eventos (@tauri-apps/api/event._unlisten) usa este global para
  // dar de baja listeners al desmontarse (p. ej. use-firebird-events al
  // navegar entre páginas). En el navegador real lo inyecta el runtime de
  // Tauri; sin él, cada navegación lanza "Cannot read properties of
  // undefined (reading 'unregisterListener')" (igual que hace mockIPC de
  // @tauri-apps/api/mocks).
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener: (_event, eventId) => {
      if (eventId && typeof window[eventId] !== "undefined") {
        delete window[eventId];
      }
    },
  };
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
