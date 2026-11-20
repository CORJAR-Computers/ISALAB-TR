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

  // ---------- Caja, facturación y recepción (flujos de dinero) ----------
  // Reproduce las reglas del backend que ejercitan los E2E: una sola caja
  // abierta, abonos que descuentan saldo, movimientos de caja solo por la
  // parte en efectivo y visitas que se facturan al hacer el checkout.
  const pad2 = (n) => String(n).padStart(2, "0");
  const stamp = () => {
    const d = new Date();
    return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())} ${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`;
  };
  const round2 = (n) => Math.round(n * 100) / 100;

  const owners = [
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
  ];

  const serviceCategories = [
    { id: 1, code: "CONSULTA", name: "Consultas", sortOrder: 1, isActive: true },
    { id: 2, code: "ESTETICA", name: "Estética", sortOrder: 2, isActive: true },
  ];

  const services = [
    {
      id: 1,
      code: "CONS-GEN",
      name: "Consulta general",
      categoryId: 1,
      categoryName: "Consultas",
      description: null,
      durationMin: 30,
      price: 60000,
      taxRate: null,
      requiresVet: true,
      requiresGroomer: false,
      speciesId: null,
      sortOrder: 1,
      isActive: true,
    },
    {
      id: 2,
      code: "BANO",
      name: "Baño y corte",
      categoryId: 2,
      categoryName: "Estética",
      description: null,
      durationMin: 60,
      price: 40000,
      taxRate: null,
      requiresVet: false,
      requiresGroomer: true,
      speciesId: null,
      sortOrder: 2,
      isActive: true,
    },
  ];

  const paymentMethods = [
    { code: "EFECTIVO", name: "Efectivo", kind: "EFECTIVO", requiresReference: false, active: true, sortOrder: 1 },
    { code: "TRANSFERENCIA", name: "Transferencia", kind: "ELECTRONICO", requiresReference: true, active: true, sortOrder: 2 },
    { code: "TARJETA", name: "Tarjeta", kind: "ELECTRONICO", requiresReference: true, active: true, sortOrder: 3 },
  ];
  const methodOf = (code) =>
    paymentMethods.find((m) => m.code === code) ?? {
      code,
      name: code,
      kind: "OTRO",
      requiresReference: false,
      active: true,
      sortOrder: 99,
    };

  const money = {
    sessions: [],
    nextSessionId: 1,
    nextMovementId: 1,
    payments: [],
    nextPaymentId: 1,
    invoices: [],
    nextInvoiceId: 1,
    visits: [],
    nextVisitId: 1,
    nextVisitServiceId: 1,
  };

  // ----- Auditoría (append-only, más reciente primero) -----
  // El backend registra LOGIN, cambios de estado y pagos vía log_audit; aquí
  // solo se replican las acciones que ejercitan los flujos E2E.
  const auditLog = {
    nextId: 3,
    entries: [
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
    push(action, details = null) {
      const id = this.nextId;
      this.nextId += 1;
      this.entries.unshift({
        id,
        userId: 1,
        username: "admin",
        action,
        details,
        createdAt: stamp(),
      });
    },
  };

  // ----- Facturas y saldos -----
  const invoiceById = (id) => money.invoices.find((i) => i.id === id);
  const paidOf = (inv) =>
    round2(
      money.payments
        .filter((p) => p.status === "APLICADO")
        .flatMap((p) => p.allocations)
        .filter((a) => a.invoiceId === inv.id)
        .reduce((acc, a) => acc + a.amount, 0),
    );
  const balanceOf = (inv) =>
    inv.status === "ANULADA"
      ? 0
      : round2(Math.max((inv.total ?? 0) - paidOf(inv), 0));
  const paymentStatusOf = (inv) => {
    if (inv.status === "ANULADA") return "ANULADA";
    const paid = paidOf(inv);
    if (paid >= (inv.total ?? 0) - 0.01) return "PAGADA";
    return paid > 0.01 ? "ABONADA" : "PENDIENTE";
  };
  const syncInvoiceStatus = (inv) => {
    if (inv.status === "ANULADA") return;
    inv.status = paymentStatusOf(inv) === "PAGADA" ? "PAGADA" : "EMITIDA";
  };
  const toInvoice = (inv) => ({
    id: inv.id,
    invoiceNumber: inv.invoiceNumber,
    patientId: inv.patientId,
    patientName: inv.patientName,
    ownerId: inv.ownerId,
    ownerName: inv.ownerName,
    ownerPhone: inv.ownerPhone ?? null,
    consultationId: inv.consultationId ?? null,
    issueDate: inv.issueDate,
    subtotal: inv.subtotal,
    taxRate: inv.taxRate,
    taxAmount: inv.taxAmount,
    total: inv.total,
    status: inv.status,
    paymentMethod: inv.paymentMethod,
    notes: inv.notes,
    items: inv.items.map((it) => ({ ...it })),
  });
  const toInvoiceListItem = (inv) => ({
    id: inv.id,
    invoiceNumber: inv.invoiceNumber,
    ownerName: inv.ownerName,
    patientName: inv.patientName,
    issueDate: inv.issueDate,
    total: inv.total,
    status: inv.status,
    paymentMethod: inv.paymentMethod,
    itemCount: inv.items.length,
    paidAmount: paidOf(inv),
    balance: balanceOf(inv),
    paymentStatus: paymentStatusOf(inv),
  });

  const pushInvoice = ({ patientId, ownerId, items, taxRate, notes, paymentMethod }) => {
    const id = money.nextInvoiceId;
    money.nextInvoiceId += 1;
    const owner = owners.find((o) => o.id === ownerId);
    const patient = patientById(patientId);
    const lines = items.map((it, idx) => ({
      id: id * 100 + idx + 1,
      description: it.description,
      quantity: it.quantity,
      unitPrice: round2(it.unitPrice),
      lineTotal: round2(it.unitPrice * it.quantity),
    }));
    const subtotal = round2(lines.reduce((acc, l) => acc + l.lineTotal, 0));
    const rate = taxRate ?? 19;
    const invoice = {
      id,
      invoiceNumber: `FAC-${String(id).padStart(6, "0")}`,
      patientId: patientId ?? null,
      patientName: patient?.name ?? null,
      ownerId,
      ownerName: owner?.fullName ?? "Consumidor final",
      ownerPhone: owner?.phone ?? null,
      consultationId: null,
      issueDate: stamp(),
      subtotal,
      taxRate: rate,
      taxAmount: round2((subtotal * rate) / 100),
      total: round2(subtotal + (subtotal * rate) / 100),
      status: "EMITIDA",
      paymentMethod: paymentMethod ?? null,
      notes: notes ?? null,
      items: lines,
    };
    money.invoices.push(invoice);
    return invoice;
  };

  const toPayment = (p) => ({
    id: p.id,
    code: p.code,
    ownerId: p.ownerId,
    ownerName: p.ownerName,
    patientId: p.patientId,
    patientName: p.patientName,
    cashSessionId: p.cashSessionId,
    receivedAt: p.receivedAt,
    total: p.total,
    status: p.status,
    notes: p.notes,
    createdByName: p.createdByName,
    lines: p.lines.map((l) => ({ ...l })),
    allocations: p.allocations.map((a) => ({
      ...a,
      invoiceNumber: invoiceById(a.invoiceId)?.invoiceNumber ?? "?",
    })),
  });

  // Factura pendiente inicial: 100.000 + 19% IVA = 119.000.
  money.invoices.push({
    id: 1,
    invoiceNumber: "FAC-000001",
    patientId: 1,
    patientName: "Rocky",
    ownerId: 10,
    ownerName: "Juan Pérez",
    ownerPhone: "3001234567",
    consultationId: null,
    issueDate: stamp(),
    subtotal: 100000,
    taxRate: 19,
    taxAmount: 19000,
    total: 119000,
    status: "EMITIDA",
    paymentMethod: "EFECTIVO",
    notes: null,
    items: [
      { id: 101, description: "Consulta general", quantity: 1, unitPrice: 60000, lineTotal: 60000 },
      { id: 102, description: "Baño y corte", quantity: 1, unitPrice: 40000, lineTotal: 40000 },
    ],
  });
  money.nextInvoiceId = 2;

  // ----- Caja -----
  const openSession = () =>
    money.sessions.find((s) => s.status === "ABIERTA") ?? null;
  const resolveSession = (id) =>
    id != null
      ? money.sessions.find((s) => s.id === id) ?? null
      : openSession();
  const sessionMovement = (session, kind, amount, reason, paymentId = null) => {
    const id = money.nextMovementId;
    money.nextMovementId += 1;
    session.movements.push({
      id,
      cashSessionId: session.id,
      kind,
      amount: round2(amount),
      paymentId,
      reason,
      createdByName: "Administrador",
      createdAt: stamp(),
    });
  };
  const sessionTotals = (session) => {
    const cashIn = round2(
      session.movements.filter((m) => m.amount > 0).reduce((a, m) => a + m.amount, 0),
    );
    const cashOut = round2(
      session.movements
        .filter((m) => m.amount < 0)
        .reduce((a, m) => a + Math.abs(m.amount), 0),
    );
    const expectedCash = round2(session.openingFloat + cashIn - cashOut);
    const applied = money.payments.filter(
      (p) => p.cashSessionId === session.id && p.status === "APLICADO",
    );
    const totalCollected = round2(applied.reduce((a, p) => a + p.total, 0));
    const byMethod = new Map();
    for (const p of applied) {
      for (const l of p.lines) {
        byMethod.set(l.methodCode, round2((byMethod.get(l.methodCode) ?? 0) + l.amount));
      }
    }
    const methods = [...byMethod.entries()].map(([code, amount]) => ({
      methodCode: code,
      methodName: methodOf(code).name,
      amount,
    }));
    const totalBilled = round2(
      money.invoices
        .filter(
          (inv) =>
            inv.status !== "ANULADA" &&
            money.payments.some(
              (p) =>
                p.status === "APLICADO" &&
                p.cashSessionId === session.id &&
                p.allocations.some((a) => a.invoiceId === inv.id),
            ),
        )
        .reduce((a, inv) => a + (inv.total ?? 0), 0),
    );
    return { cashIn, cashOut, expectedCash, totalCollected, totalBilled, methods, applied };
  };
  const toCashSession = (session) => {
    const t = sessionTotals(session);
    return {
      id: session.id,
      code: session.code,
      openedAt: session.openedAt,
      openedBy: session.openedBy,
      openedByName: session.openedByName,
      openingFloat: session.openingFloat,
      status: session.status,
      closedAt: session.closedAt,
      closedByName: session.closedByName,
      expectedCash: t.expectedCash,
      countedCash: session.countedCash,
      difference: session.difference,
      totalBilled: t.totalBilled,
      totalCollected: t.totalCollected,
      notes: session.notes,
    };
  };
  const buildSummary = (session) => {
    const t = sessionTotals(session);
    return {
      session: toCashSession(session),
      openingFloat: session.openingFloat,
      expectedCash: t.expectedCash,
      countedCash: session.countedCash,
      difference: session.difference,
      totalBilled: t.totalBilled,
      totalCollected: t.totalCollected,
      cashIn: t.cashIn,
      cashOut: t.cashOut,
      paymentsCount: t.applied.length,
      methods: t.methods,
      movements: session.movements.map((m) => ({ ...m })),
      countLines: (session.countLines ?? []).map((l) => ({ ...l })),
    };
  };

  const registerPayment = (input) => {
    const session = openSession();
    if (!session) {
      throw { type: "Validation", data: "Abre una caja antes de cobrar" };
    }
    const total = round2((input.lines ?? []).reduce((a, l) => a + l.amount, 0));
    if (total <= 0) {
      throw { type: "Validation", data: "El pago no tiene importe" };
    }
    const allocations = (input.allocations ?? []).map((a) => {
      const inv = invoiceById(a.invoiceId);
      if (!inv) throw { type: "NotFound", data: "Factura no encontrada" };
      if (inv.status === "ANULADA") {
        throw { type: "Validation", data: "La factura está anulada" };
      }
      if (a.amount > balanceOf(inv) + 0.01) {
        throw {
          type: "Validation",
          data: `El abono supera el saldo de ${inv.invoiceNumber}`,
        };
      }
      return { id: a.invoiceId, invoiceId: a.invoiceId, amount: round2(a.amount) };
    });
    const id = money.nextPaymentId;
    money.nextPaymentId += 1;
    const payment = {
      id,
      code: `REC-${String(id).padStart(6, "0")}`,
      ownerId: input.ownerId,
      ownerName: owners.find((o) => o.id === input.ownerId)?.fullName ?? "Consumidor final",
      patientId: input.patientId ?? null,
      patientName: patientById(input.patientId)?.name ?? null,
      cashSessionId: session.id,
      receivedAt: stamp(),
      total,
      status: "APLICADO",
      notes: input.notes ?? null,
      createdByName: "Administrador",
      lines: (input.lines ?? []).map((l, idx) => {
        const m = methodOf(l.methodCode);
        return {
          id: id * 10 + idx + 1,
          methodCode: m.code,
          methodName: m.name,
          methodKind: m.kind,
          amount: round2(l.amount),
          reference: l.reference ?? null,
        };
      }),
      allocations,
    };
    money.payments.push(payment);
    for (const line of payment.lines) {
      if (line.methodKind === "EFECTIVO") {
        sessionMovement(
          session,
          "PAGO_EFECTIVO",
          line.amount,
          `Pago ${payment.code}`,
          payment.id,
        );
      }
    }
    for (const a of allocations) syncInvoiceStatus(invoiceById(a.invoiceId));
    auditLog.push(
      "PAYMENT_REGISTERED",
      `Pago ${payment.code} por ${payment.total.toFixed(2)} aplicado a ${allocations.length} factura(s)`,
    );
    return toPayment(payment);
  };

  const voidPayment = (paymentId, reason) => {
    const payment = money.payments.find((p) => p.id === paymentId);
    if (!payment) throw { type: "NotFound", data: "Pago no encontrado" };
    if (payment.status === "ANULADO") {
      throw { type: "Validation", data: "El pago ya está anulado" };
    }
    if (!(reason ?? "").trim()) {
      throw { type: "Validation", data: "El motivo de la anulación es obligatorio" };
    }
    const session = resolveSession(payment.cashSessionId);
    if (!session || session.status !== "ABIERTA") {
      throw { type: "Validation", data: "La caja del pago ya está cerrada" };
    }
    payment.status = "ANULADO";
    for (const line of payment.lines) {
      if (line.methodKind === "EFECTIVO") {
        sessionMovement(
          session,
          "DEVOLUCION",
          -line.amount,
          `Anulación ${payment.code}: ${reason}`,
          payment.id,
        );
      }
    }
    for (const a of payment.allocations) syncInvoiceStatus(invoiceById(a.invoiceId));
    auditLog.push(
      "PAYMENT_VOIDED",
      `Pago ${payment.code} anulado por ${payment.total.toFixed(2)}: ${reason.trim()}`,
    );
    return toPayment(payment);
  };

  // ----- Visitas -----
  const visitById = (id) => money.visits.find((v) => v.id === id);
  const visitTotal = (visit) =>
    round2(
      visit.services
        .filter((l) => l.status !== "CANCELADA")
        .reduce((acc, l) => acc + (l.unitPrice ?? 0) * l.quantity, 0),
    );
  const toVisit = (visit) => ({
    id: visit.id,
    code: visit.code,
    patientId: visit.patientId,
    patientName: visit.patientName,
    ownerId: visit.ownerId,
    ownerName: visit.ownerName,
    appointmentId: null,
    arrivedAt: visit.arrivedAt,
    priority: visit.priority,
    status: visit.status,
    notes: visit.notes,
    createdByName: "Administrador",
    services: visit.services.map((l) => ({ ...l, visitId: visit.id })),
  });
  const toVisitListItem = (visit) => ({
    id: visit.id,
    code: visit.code,
    patientId: visit.patientId,
    patientName: visit.patientName,
    ownerId: visit.ownerId,
    ownerName: visit.ownerName,
    arrivedAt: visit.arrivedAt,
    priority: visit.priority,
    status: visit.status,
    notes: visit.notes,
    servicesSummary: visit.services
      .filter((l) => l.status !== "CANCELADA")
      .map((l) => l.serviceName)
      .join(" + "),
    estimatedTotal: visitTotal(visit),
  });
  const visitLine = (visit, service, input) => {
    const id = money.nextVisitServiceId;
    money.nextVisitServiceId += 1;
    return {
      id,
      visitId: visit.id,
      serviceId: service.id,
      serviceName: service.name,
      serviceCode: service.code,
      categoryId: service.categoryId,
      categoryName: service.categoryName,
      professionalId: input.professionalId ?? null,
      professionalName: null,
      quantity: input.quantity ?? 1,
      unitPrice: service.price,
      taxRate: service.taxRate,
      status: "PENDIENTE",
      startedAt: null,
      finishedAt: null,
      notes: input.notes ?? null,
      seq: visit.services.length + 1,
    };
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
        visitsToday: money.visits.length,
        visitsWaiting: money.visits.filter((v) => v.status === "EN_ESPERA").length,
        visitsUrgentWaiting: money.visits.filter(
          (v) => v.status === "EN_ESPERA" && v.priority === "URGENTE",
        ).length,
        avgWaitMinutes: 0,
        appointmentsToday: 0,
        groomingSessionsToday: 0,
        revenueByCategory: [],
        // Fase D de abonos/caja: cobrado, cartera y estado de la caja.
        collectedToday: round2(
          money.payments
            .filter((p) => p.status === "APLICADO")
            .reduce((a, p) => a + p.total, 0),
        ),
        collectedMonth: round2(
          money.payments
            .filter((p) => p.status === "APLICADO")
            .reduce((a, p) => a + p.total, 0),
        ),
        receivableTotal: round2(
          money.invoices.reduce((a, i) => a + balanceOf(i), 0),
        ),
        receivableCount: money.invoices.filter((i) => balanceOf(i) > 0.01).length,
        cashSessionOpen: openSession() != null,
        lastCashDifference:
          money.sessions.filter((s) => s.status === "CERRADA").length > 0
            ? money.sessions.filter((s) => s.status === "CERRADA").at(-1).difference
            : null,
        upcomingConsultations: [],
        upcomingSurgeries: [],
        upcomingVaccines: [],
        recentSamples: list.slice(-5).reverse(),
        upcomingAppointments: [],
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
        // Rango definido por el veterinario: precedencia sobre el catálogo.
        const hasCustom = item.customRefMin != null || item.customRefMax != null;
        let status;
        let refMin;
        let refMax;
        if (hasCustom) {
          refMin = item.customRefMin ?? null;
          refMax = item.customRefMax ?? null;
          if (refMax != null && item.value > refMax) status = "ALTO";
          else if (refMin != null && item.value < refMin) status = "BAJO";
          else status = "NORMAL";
        } else {
          ({ status, refMin, refMax } = resultMeta(a.id, item.value));
        }
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
          customRefMin: hasCustom ? refMin : null,
          customRefMax: hasCustom ? refMax : null,
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
    list_owners: () => owners,
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
    list_audit_log: (args = {}) => {
      let rows = [...auditLog.entries].sort((a, b) => b.id - a.id);
      if (args.username) {
        const q = String(args.username).toLowerCase();
        rows = rows.filter((r) => r.username.toLowerCase().includes(q));
      }
      if (args.action) rows = rows.filter((r) => r.action === args.action);
      const offset = args.offset ?? 0;
      const limit = args.limit ?? rows.length;
      return rows.slice(offset, offset + limit).map((r) => ({ ...r }));
    },
    list_consultations: () => [],
    count_consultations: () => [],
    list_surgeries: () => [],
    count_surgeries: () => [],
    list_vaccines: () => [],
    // Catálogo de tipos de vacuna (seed de 0002_seed_catalog.sql). El diálogo
    // de nueva vacuna lo consulta al montar la página de Vacunación.
    list_vaccine_types: () => [
      { id: 1, code: "RABIA", name: "Rabia" },
      { id: 2, code: "POLI_CANINA", name: "Polivalente canina (M2)" },
      { id: 3, code: "POLI_FELINA", name: "Polivalente felina (TCRP)" },
      { id: 4, code: "LEPTO", name: "Leptospirosis" },
      { id: 5, code: "TOS_PERRERAS", name: "Tos de las perreras" },
    ],
    // ---- Facturación, pagos y reportes de dinero ----
    list_invoices: (args) => {
      let list = money.invoices.map(toInvoiceListItem);
      if (args.status) list = list.filter((i) => i.paymentStatus === args.status);
      const q = (args.search ?? "").toLowerCase();
      if (q) {
        list = list.filter(
          (i) =>
            i.invoiceNumber.toLowerCase().includes(q) ||
            i.ownerName.toLowerCase().includes(q) ||
            (i.patientName ?? "").toLowerCase().includes(q),
        );
      }
      return list;
    },
    count_invoices: () => {
      const rows = [];
      for (const status of ["PENDIENTE", "ABONADA", "PAGADA", "ANULADA"]) {
        const count = money.invoices.filter(
          (i) => paymentStatusOf(i) === status,
        ).length;
        if (count > 0) rows.push({ status, count });
      }
      return rows;
    },
    get_invoice: (args) => {
      const inv = invoiceById(args.id);
      return inv ? toInvoice(inv) : null;
    },
    create_invoice: (args) => {
      const input = args.input ?? {};
      return toInvoice(
        pushInvoice({
          patientId: input.patientId ?? null,
          ownerId: input.ownerId,
          items: input.items ?? [],
          taxRate: input.taxRate ?? null,
          notes: input.notes ?? null,
          paymentMethod: input.paymentMethod ?? null,
        }),
      );
    },
    set_invoice_status: (args) => {
      const inv = invoiceById(args.id);
      if (!inv) throw { type: "NotFound", data: "Factura no encontrada" };
      if (args.status !== "ANULADA") {
        throw {
          type: "Validation",
          data: "Solo se puede anular; el estado PAGADA lo determina el cobro",
        };
      }
      if (inv.status === "ANULADA") {
        throw { type: "Validation", data: "La factura ya está anulada" };
      }
      const active = money.payments.some(
        (p) =>
          p.status === "APLICADO" &&
          p.allocations.some((a) => a.invoiceId === inv.id),
      );
      if (active) {
        throw {
          type: "Validation",
          data: "La factura tiene pagos vigentes: anúlalos primero",
        };
      }
      inv.status = "ANULADA";
      // Misma acción que `set_invoice_status` en el backend.
      auditLog.push("INVOICE_STATUS_CHANGE", `Factura ${inv.id} → estado ANULADA`);
      return toInvoice(inv);
    },
    get_invoice_balance: (args) => {
      const inv = invoiceById(args.invoiceId);
      if (!inv) throw { type: "NotFound", data: "Factura no encontrada" };
      return {
        invoiceId: inv.id,
        invoiceNumber: inv.invoiceNumber,
        total: inv.total ?? 0,
        paidAmount: paidOf(inv),
        balance: balanceOf(inv),
        paymentStatus: paymentStatusOf(inv),
        status: inv.status,
      };
    },
    list_invoice_payments: (args) =>
      money.payments
        .filter((p) => p.allocations.some((a) => a.invoiceId === args.invoiceId))
        .map(toPayment),
    list_payment_methods: (args) =>
      paymentMethods.filter((m) => (args.includeInactive ? true : m.active)),
    register_payment: (args) => registerPayment(args.input ?? {}),
    void_payment: (args) => voidPayment(args.paymentId, args.reason),
    get_billing_report: (args) => {
      const from = args.from ?? "0000-01-01";
      const to = args.to ?? "9999-12-31";
      const inRange = (day) => day >= from && day <= to;
      const billed = money.invoices.filter(
        (i) => i.status !== "ANULADA" && inRange(i.issueDate.slice(0, 10)),
      );
      const received = money.payments.filter(
        (p) => p.status === "APLICADO" && inRange(p.receivedAt.slice(0, 10)),
      );
      const byMethod = new Map();
      for (const p of received) {
        for (const l of p.lines) {
          byMethod.set(l.methodCode, round2((byMethod.get(l.methodCode) ?? 0) + l.amount));
        }
      }
      return {
        fromDate: from,
        toDate: to,
        billedTotal: round2(billed.reduce((a, i) => a + (i.total ?? 0), 0)),
        billedTax: round2(billed.reduce((a, i) => a + (i.taxAmount ?? 0), 0)),
        billedCount: billed.length,
        collectedTotal: round2(received.reduce((a, p) => a + p.total, 0)),
        collectedCount: received.length,
        outstandingTotal: round2(billed.reduce((a, i) => a + balanceOf(i), 0)),
        methods: [...byMethod.entries()].map(([code, amount]) => ({
          methodCode: code,
          methodName: methodOf(code).name,
          amount,
        })),
      };
    },
    generate_recibo_invoice: (args) => {
      const inv = invoiceById(args.invoiceId);
      if (!inv) throw { type: "NotFound", data: "Factura no encontrada" };
      return {
        path: `C:/mock/recibo-${inv.invoiceNumber}.pdf`,
        fileName: `ISALAB_Recibo_${inv.invoiceNumber}.pdf`,
        sampleCode: inv.invoiceNumber,
        generatedAt: stamp(),
      };
    },

    // ---- Caja ----
    get_open_cash_session: () => {
      const session = openSession();
      return session ? toCashSession(session) : null;
    },
    open_cash_session: (args) => {
      if (openSession()) {
        throw { type: "Validation", data: "Ya hay una caja abierta" };
      }
      const id = money.nextSessionId;
      money.nextSessionId += 1;
      const session = {
        id,
        code: `CAJ-${String(id).padStart(8, "0")}`,
        openedAt: stamp(),
        openedBy: 1,
        openedByName: "Administrador",
        openingFloat: round2(args.openingFloat ?? 0),
        status: "ABIERTA",
        closedAt: null,
        closedByName: null,
        countedCash: null,
        difference: null,
        notes: args.notes ?? null,
        movements: [],
      };
      money.sessions.push(session);
      return toCashSession(session);
    },
    add_cash_movement: (args) => {
      const session = openSession();
      if (!session) {
        throw { type: "Validation", data: "No hay una caja abierta" };
      }
      const input = args.input ?? {};
      const amount = round2(Math.abs(input.amount ?? 0));
      if (amount <= 0 || !(input.reason ?? "").trim()) {
        throw { type: "Validation", data: "Importe y motivo son obligatorios" };
      }
      const signed =
        input.kind === "AJUSTE" ? round2(input.amount) : -amount;
      sessionMovement(session, input.kind, signed, input.reason.trim());
      return session.movements[session.movements.length - 1];
    },
    list_cash_movements: (args) => {
      const session = resolveSession(args.sessionId);
      return session ? session.movements.map((m) => ({ ...m })) : [];
    },
    get_cash_session_summary: (args) => {
      const session = resolveSession(args.sessionId);
      return session ? buildSummary(session) : null;
    },
    close_cash_session: (args) => {
      const session = openSession();
      if (!session) {
        throw { type: "Validation", data: "No hay una caja abierta" };
      }
      const input = args.input ?? {};
      const countLines = (input.countLines ?? []).map((l) => ({
        denomination: l.denomination,
        quantity: l.quantity,
        subtotal: round2(l.denomination * l.quantity),
      }));
      const countedCash = round2(
        countLines.reduce((a, l) => a + l.subtotal, 0),
      );
      if (countLines.length === 0) {
        throw {
          type: "Validation",
          data: "Registra al menos una denominación en el arqueo",
        };
      }
      const { expectedCash } = sessionTotals(session);
      session.status = "CERRADA";
      session.closedAt = stamp();
      session.closedByName = "Administrador";
      session.countedCash = countedCash;
      session.difference = round2(countedCash - expectedCash);
      session.countLines = countLines;
      session.notes = input.notes ?? session.notes;
      return buildSummary(session);
    },
    list_cash_sessions: (args) =>
      [...money.sessions]
        .sort((a, b) => b.id - a.id)
        .slice(0, args.limit ?? 30)
        .map(toCashSession),
    generate_cash_close_pdf: (args) => {
      const session = resolveSession(args.sessionId);
      return {
        path: `C:/mock/cierre-${session?.code ?? "CAJA"}.pdf`,
        fileName: `ISALAB_Cierre_${session?.code ?? "CAJA"}.pdf`,
        sampleCode: session?.code ?? "CAJA",
        generatedAt: stamp(),
      };
    },

    // ---- Recepción de pacientes (visitas) ----
    list_service_categories: () => serviceCategories,
    list_all_service_categories: () => serviceCategories,
    list_services: (args) => {
      const q = (args.search ?? "").toLowerCase();
      return services.filter((s) => {
        if (!args.includeInactive && !s.isActive) return false;
        if (args.categoryId != null && s.categoryId !== args.categoryId) return false;
        if (q && !s.name.toLowerCase().includes(q)) return false;
        return true;
      });
    },
    list_visits: (args) => {
      let list = [...money.visits].sort((a, b) => b.id - a.id);
      if (args.date) list = list.filter((v) => v.arrivedAt.slice(0, 10) === args.date);
      if (args.status) list = list.filter((v) => v.status === args.status);
      const q = (args.search ?? "").toLowerCase();
      if (q) {
        list = list.filter(
          (v) =>
            v.code.toLowerCase().includes(q) ||
            v.patientName.toLowerCase().includes(q) ||
            v.ownerName.toLowerCase().includes(q),
        );
      }
      return list.map(toVisitListItem);
    },
    get_visit: (args) => {
      const visit = visitById(args.id);
      return visit ? toVisit(visit) : null;
    },
    create_visit: (args) => {
      const input = args.input ?? {};
      const patient = patientById(input.patientId);
      if (!patient) throw { type: "Validation", data: "Paciente inválido" };
      if (!(input.services ?? []).length) {
        throw { type: "Validation", data: "Agrega al menos un servicio" };
      }
      const id = money.nextVisitId;
      money.nextVisitId += 1;
      const visit = {
        id,
        code: `V-2026-${String(id).padStart(4, "0")}`,
        patientId: patient.id,
        patientName: patient.name,
        ownerId: patient.ownerId,
        ownerName: patient.ownerName,
        // Mismo criterio que el backend: ARRIVED_AT se sella con la fecha
        // local del equipo (CURRENT_TIMESTAMP de Firebird), no con UTC.
        arrivedAt: stamp(),
        priority: input.priority ?? "NORMAL",
        status: "EN_ESPERA",
        notes: input.notes ?? null,
        services: [],
      };
      for (const line of input.services) {
        const service = services.find((s) => s.id === line.serviceId);
        if (!service) throw { type: "Validation", data: "Servicio inválido" };
        visit.services.push(visitLine(visit, service, line));
      }
      money.visits.push(visit);
      return toVisit(visit);
    },
    set_visit_status: (args) => {
      const visit = visitById(args.id);
      if (!visit) throw { type: "NotFound", data: "Visita no encontrada" };
      if (visit.status === "FACTURADA" && args.status !== "FACTURADA") {
        throw { type: "Validation", data: "Una visita facturada no cambia de estado" };
      }
      visit.status = args.status;
      return toVisit(visit);
    },
    add_visit_service: (args) => {
      const visit = visitById(args.visitId);
      if (!visit) throw { type: "NotFound", data: "Visita no encontrada" };
      const service = services.find((s) => s.id === args.input?.serviceId);
      if (!service) throw { type: "Validation", data: "Servicio inválido" };
      const line = visitLine(visit, service, args.input ?? {});
      visit.services.push(line);
      return { ...line };
    },
    remove_visit_service: (args) => {
      const visit = visitById(args.visitId);
      if (!visit) throw { type: "NotFound", data: "Visita no encontrada" };
      visit.services = visit.services.filter((l) => l.id !== args.lineId);
      return null;
    },
    update_visit_service_status: (args) => {
      const visit = visitById(args.visitId);
      const line = visit?.services.find((l) => l.id === args.lineId);
      if (!visit || !line) throw { type: "NotFound", data: "Línea no encontrada" };
      line.status = args.input?.status ?? line.status;
      if (line.status === "EN_PROCESO" && !line.startedAt) line.startedAt = stamp();
      if (line.status === "COMPLETADA" && !line.finishedAt) line.finishedAt = stamp();
      return { ...line };
    },
    checkout_visit: (args) => {
      const visit = visitById(args.visitId);
      if (!visit) throw { type: "NotFound", data: "Visita no encontrada" };
      if (visit.status !== "FINALIZADA") {
        throw {
          type: "Validation",
          data: "Solo se cobra una visita finalizada",
        };
      }
      const lines = visit.services.filter((l) => l.status !== "CANCELADA");
      if (lines.length === 0) {
        throw { type: "Validation", data: "La visita no tiene servicios activos" };
      }
      const invoice = pushInvoice({
        patientId: visit.patientId,
        ownerId: visit.ownerId,
        items: lines.map((l) => ({
          description: l.serviceName,
          quantity: l.quantity,
          unitPrice: l.unitPrice ?? 0,
        })),
        taxRate: 19,
        notes: args.notes ?? null,
        paymentMethod: args.paymentMethod ?? null,
      });
      visit.status = "FACTURADA";
      return toInvoice(invoice);
    },
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
