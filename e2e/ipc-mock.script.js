// Mock del runtime de Tauri para los tests E2E (Playwright).
//
// Se inyecta con page.addInitScript ANTES de que cargue la app: define
// window.__TAURI_INTERNALS__ (que es lo que usa @tauri-apps/api/core#invoke)
// y responde los comandos del flujo smoke con un estado en memoria.
// Un comando sin handler NO devuelve null: devolver null hacía que la página
// pintara un estado vacío y el test pasara sin cubrir la llamada. Ahora lanza
// un error legible para que falle el spec que lo ejerce; si los E2E no deben
// ejercitarlo, se declara en e2e/ipc-mock.allowlist.mjs (lo verifica
// `npm run check:e2e-mock`).
//
// Inyección de fallos (E2E): `window.__E2E_IPC_FAULTS__.set({...})` hace que
// los comandos seleccionados rechacen como lo haría el backend con la base
// caída (AppError { type, data }), para probar los estados de error y el
// reintento de la UI de verdad. Configuración:
//   { all: true }                  → todos los comandos
//   { groups: ["reception"] }       → un grupo de FAULT_GROUPS
//   { commands: ["get_session"] }   → un comando concreto
//   { prefixes: ["list_"] }         → por prefijo de comando
//   { remaining: 1 }               → solo las próximas N llamadas
//   { errorType?, message? }       → forma del error (por defecto Database)
// La configuración vive en localStorage (clave "e2e:ipc-faults") para poder
// fijarla con addInitScript ANTES de que arranque la app y sobrevivir a un
// reload. `__E2E_IPC_FAULTS__.stats()` cuenta llamadas por comando, útil para
// esperar el reintento automático de react-query antes de simular la vuelta
// de la base.
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

  // tubeType/anticoagulant/minVolumeMl replican las columnas de la tabla
  // (todas nullable); antes faltaban y la tabla quedaba sin datos.
  const sampleTypes = [
    {
      id: 1,
      code: "BLOOD",
      name: "Sangre",
      tubeType: "EDTA",
      anticoagulant: "EDTA K2",
      minVolumeMl: 1,
    },
    {
      id: 2,
      code: "SERUM",
      name: "Suero",
      tubeType: "Tapa roja",
      anticoagulant: null,
      minVolumeMl: 2,
    },
    {
      id: 3,
      code: "URINE",
      name: "Orina",
      tubeType: "Estéril",
      anticoagulant: null,
      minVolumeMl: 5,
    },
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

  // ---------- Fecha local (sin años fijos) ----------
  // El mock sella con la fecha local del equipo, igual que el backend
  // (CURRENT_TIMESTAMP de Firebird). Ningún año se escribe a mano: los códigos
  // M-AAAA-NNNN / V-AAAA-NNNN y los sellos se derivan del reloj, que los tests
  // de frontera fijan con page.clock (e2e/date-boundaries.spec.ts).
  const pad2 = (n) => String(n).padStart(2, "0");
  const localDate = (d = new Date()) =>
    `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
  const stampAt = (d) =>
    `${localDate(d)} ${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`;
  const stamp = () => stampAt(new Date());
  const currentYear = () => new Date().getFullYear();
  /** Sello de hace `days` días (a la hora indicada) para los datos sembrados. */
  const daysAgo = (days, hours = 9, minutes = 0) => {
    const d = new Date();
    d.setDate(d.getDate() - days);
    d.setHours(hours, minutes, 0, 0);
    return stampAt(d);
  };

  // Historial y notificaciones sembrados para la primera muestra del flujo
  // (la que crea el smoke test recibe id 1). Ejercitan los EventRow y
  // NotificationRow de los diálogos apilados del detalle.
  const sampleEvents = [
    { id: 1, sampleId: 1, eventType: "REJECTED", username: "admin", reason: "Tubo sin etiquetar", createdAt: daysAgo(2, 10, 15) },
    { id: 2, sampleId: 1, eventType: "REOPENED", username: "admin", reason: null, createdAt: daysAgo(2, 11, 40) },
  ];
  const notificationsLog = [
    { id: 1, resultId: null, sampleId: 1, channel: "EMAIL", recipientName: "Juan Pérez", recipientAddress: "juan.perez@example.com", status: "SENT", sentAt: daysAgo(2, 12, 5), ackedAt: null, ackedBy: null, note: null, createdAt: daysAgo(2, 12, 5) },
    { id: 2, resultId: null, sampleId: 1, channel: "MANUAL", recipientName: null, recipientAddress: null, status: "ACKNOWLEDGED", sentAt: null, ackedAt: daysAgo(2, 12, 20), ackedBy: "Dra. Ana Pérez", note: null, createdAt: daysAgo(2, 12, 20) },
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
  const sampleCode = () => `M-${currentYear()}-${pad4(nextSampleId)}`;
  const patientById = (id) => patients.find((p) => p.id === id);
  const serviceById = (id) => services.find((s) => s.id === id);
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
      patientCode: p?.code ?? "?",
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
      criticalCount: s.results.filter(
        (r) => r.status === "CRITICO" || r.status === "CRITICAL",
      ).length,
      qualityIndex: s.qualityIndex ?? null,
      qualitySeverity: s.qualitySeverity ?? null,
      rejectionReason: s.rejectionReason ?? null,
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
  const round2 = (n) => Math.round(n * 100) / 100;

  // ----- Usuarios, roles y capacidades (espejo de src-tauri/src/auth.rs) ---
  // El mock de login devuelve el rol REAL del usuario: antes respondía siempre
  // ADMIN, así que los E2E no podían observar ni el filtrado del menú por rol
  // ni los 403 del backend. Las capacidades replican `capabilities_for_role`
  // (su paridad con src/hooks/use-permissions.ts la comprueba
  // src/hooks/use-permissions.test.ts).
  const mockUsers = [
    { id: 1, username: "admin", password: "admin123", fullName: "Administrador", role: "ADMIN" },
    { id: 2, username: "mv.perez", password: "vet123", fullName: "Dra. Ana Pérez", role: "VETERINARIO" },
    { id: 3, username: "recepcion", password: "recepcion123", fullName: "Laura Gómez", role: "RECEPCION" },
    { id: 4, username: "auxiliar", password: "auxiliar123", fullName: "Carlos Ruiz", role: "AUXILIAR" },
    { id: 5, username: "peluquero", password: "peluquero123", fullName: "Diego Santos", role: "PELUQUERO" },
  ];

  const NO_CAPABILITIES = {
    receive: false,
    clinical: false,
    groom: false,
    bill: false,
    stay: false,
    inventory: false,
    admin: false,
  };
  const CAPABILITIES_BY_ROLE = {
    ADMIN: { receive: true, clinical: true, groom: true, bill: true, stay: true, inventory: true, admin: true },
    VETERINARIO: { receive: true, clinical: true, groom: false, bill: true, stay: true, inventory: false, admin: false },
    AUXILIAR: { receive: true, clinical: false, groom: true, bill: true, stay: true, inventory: true, admin: false },
    RECEPCION: { receive: true, clinical: false, groom: false, bill: true, stay: true, inventory: true, admin: false },
    PELUQUERO: { receive: false, clinical: false, groom: true, bill: false, stay: false, inventory: false, admin: false },
  };
  const capabilitiesOf = (role) => CAPABILITIES_BY_ROLE[role] ?? NO_CAPABILITIES;

  // Usuario de la sesión activa (login/logout lo mantienen).
  let sessionUser = null;

  // Mismos mensajes que los require_* de auth.rs, para que la UI y los tests
  // vean el rechazo tal como lo produce el backend.
  const requireCapability = (capability, message) => {
    if (!capabilitiesOf(sessionUser?.role ?? "GUEST")[capability]) {
      throw { type: "Forbidden", data: message };
    }
  };
  const requireAdmin = () =>
    requireCapability(
      "admin",
      "Solo el administrador puede realizar esta acción",
    );
  const requireVetOrAdmin = () =>
    requireCapability(
      "clinical",
      "Se requieren permisos de Veterinario o Administrador para esta acción",
    );
  const requireReceive = () =>
    requireCapability(
      "receive",
      "Se requiere rol de Recepción, Auxiliar, Veterinario o Administrador",
    );
  const requireBill = () =>
    requireCapability(
      "bill",
      "Se requiere capacidad de facturación (Recepción, Auxiliar, Veterinario o Administrador)",
    );

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

  // ----- Agenda de citas -----
  // Dos citas de hoy: la de Rocky está PROGRAMADA (el clic hace check-in y crea
  // la visita vinculada) y la de Luna ya está ATENDIDA (el clic abre su
  // historial clínico). Las horas se calculan al arrancar el mock para que
  // caigan siempre en la semana que pinta la agenda.
  const consulta = services.find((s) => s.code === "CONS-GEN");
  /** Estados de origen que admite cada transición (la misma máquina que el backend). */
  const APPOINTMENT_FROM = {
    CONFIRMADA: ["PROGRAMADA"],
    EN_ATENCION: ["PROGRAMADA", "CONFIRMADA"],
    ATENDIDA: ["EN_ATENCION"],
    NO_ASISTIO: ["PROGRAMADA", "CONFIRMADA", "EN_ATENCION"],
    CANCELADA: ["PROGRAMADA", "CONFIRMADA", "EN_ATENCION"],
  };
  /** `YYYY-MM-DDTHH:MM` (lo que da un datetime-local) → `YYYY-MM-DD HH:MM:SS`. */
  const asStamp = (value) => {
    const spaced = value.replace("T", " ");
    return spaced.length === 16 ? `${spaced}:00` : spaced;
  };
  /** Cita completa (`Appointment`); la usan las sembradas y las creadas. */
  const buildAppointment = ({
    id,
    patient,
    service,
    scheduledAt,
    status,
    professionalId = null,
    durationMin,
    notes = null,
    createdAt = scheduledAt,
  }) => ({
    id,
    code: `C-${currentYear()}-${String(id).padStart(4, "0")}`,
    patientId: patient.id,
    patientName: patient.name,
    ownerId: patient.ownerId,
    ownerName: patient.ownerName,
    serviceId: service.id,
    serviceName: service.name,
    serviceCode: service.code,
    categoryId: service.categoryId,
    categoryName: service.categoryName,
    professionalId,
    professionalName: null,
    scheduledAt,
    durationMin: durationMin ?? service.durationMin,
    status,
    notes,
    createdAt,
    updatedAt: createdAt,
  });
  const seedAppointment = (id, patientId, hours, status) =>
    buildAppointment({
      id,
      patient: patientById(patientId),
      service: consulta,
      scheduledAt: daysAgo(0, hours, 0),
      status,
    });
  const agenda = {
    appointments: [
      seedAppointment(1, 1, 9, "PROGRAMADA"),
      seedAppointment(2, 2, 11, "ATENDIDA"),
    ],
    nextId: 3,
  };
  /** Fila del listado: `AppointmentListItem` deja fuera estos campos. */
  const toAppointmentListItem = ({
    serviceCode: _serviceCode,
    professionalId: _professionalId,
    notes: _notes,
    createdAt: _createdAt,
    updatedAt: _updatedAt,
    ...row
  }) => row;
  const appointmentById = (id) => agenda.appointments.find((a) => a.id === id);
  /** Cambia el estado, validando la transición como el backend. */
  const moveAppointment = (appt, status, notes) => {
    if (!(APPOINTMENT_FROM[status] ?? []).includes(appt.status)) {
      throw {
        type: "Validation",
        data: `Transición de estado inválida: ${appt.status} → ${status}`,
      };
    }
    appt.status = status;
    // El backend hace COALESCE(?, NOTES): las notas solo se tocan si llegan.
    if (notes != null) appt.notes = notes;
    appt.updatedAt = stamp();
    return { ...appt };
  };

  // Actividad clínica sembrada para que el historial tenga datos sin depender
  // de crear una visita durante el test. No vive en `money.visits` a propósito:
  // el tablero de recepción lo usan otros specs y su estado inicial sigue vacío.
  const clinicalFixtures = {
    visits: [
      {
        id: 900,
        code: `V-${currentYear()}-0900`,
        patientId: 2,
        patientName: "Luna",
        ownerId: 10,
        ownerName: "Juan Pérez",
        arrivedAt: daysAgo(3, 10, 0),
        priority: "NORMAL",
        status: "FINALIZADA",
        notes: null,
        servicesSummary: consulta.name,
        estimatedTotal: consulta.price,
      },
    ],
    groomingSessions: [
      {
        id: 1,
        visitServiceId: 1,
        visitId: 900,
        serviceName: "Baño completo",
        patientId: 2,
        patientName: "Luna",
        ownerName: "Juan Pérez",
        groomerId: null,
        groomerName: "Andrea",
        style: "Corte higiénico",
        bathType: "Medicado",
        coatCondition: "Bueno",
        mattingLevel: 0,
        weightKg: 12.5,
        behavior: "Tranquilo",
        productsUsed: null,
        startedAt: daysAgo(3, 11, 0),
        finishedAt: daysAgo(3, 12, 0),
        nextSuggestedAt: null,
        notes: null,
        createdAt: daysAgo(3, 10, 45),
        updatedAt: daysAgo(3, 12, 0),
      },
    ],
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
        createdAt: daysAgo(1, 8, 0),
      },
      {
        id: 2,
        userId: 1,
        username: "admin",
        action: "SETTINGS_CHANGED",
        details: "Datos de la clínica actualizados",
        createdAt: daysAgo(1, 8, 15),
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
    appointmentId: visit.appointmentId ?? null,
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
  // ---------- Inyección de fallos (E2E) ----------
  // Grupos con nombre para que los specs no mantengan listas de comandos.
  // Un grupo se queda corto con el tiempo: el fallo simplemente no aplica.
  const FAULT_GROUPS = {
    session: ["get_session", "login", "logout"],
    dashboard: ["get_dashboard_stats"],
    reception: [
      "list_visits",
      "get_visit",
      "create_visit",
      "set_visit_status",
      "add_visit_service",
      "remove_visit_service",
      "update_visit_service_status",
      "checkout_visit",
    ],
    patients: ["list_patients", "get_patient", "global_search"],
    samples: [
      "list_samples",
      "get_sample",
      "count_samples",
      "list_sample_events",
      "get_worklist",
    ],
    invoices: [
      "list_invoices",
      "count_invoices",
      "get_invoice",
      "get_invoice_balance",
      "list_invoice_payments",
    ],
    cash: [
      "get_open_cash_session",
      "open_cash_session",
      "add_cash_movement",
      "list_cash_movements",
      "get_cash_session_summary",
      "close_cash_session",
    ],
  };

  const FAULTS_KEY = "e2e:ipc-faults";
  const DB_DOWN_MESSAGE =
    "No se pudo conectar con la base de datos (Firebird no responde).";

  const readFaults = () => {
    try {
      const raw = window.localStorage.getItem(FAULTS_KEY);
      return raw ? JSON.parse(raw) : {};
    } catch {
      return {};
    }
  };

  const writeFaults = (config) => {
    try {
      window.localStorage.setItem(FAULTS_KEY, JSON.stringify(config ?? {}));
    } catch {
      // localStorage no disponible: el test corre sin fallos, no se rompe aquí.
    }
  };

  const faultMatches = (config, cmd) => {
    if (!config) return false;
    if (config.all) return true;
    if (Array.isArray(config.commands) && config.commands.includes(cmd)) {
      return true;
    }
    if (
      Array.isArray(config.groups) &&
      config.groups.some((group) => (FAULT_GROUPS[group] ?? []).includes(cmd))
    ) {
      return true;
    }
    if (
      Array.isArray(config.prefixes) &&
      config.prefixes.some((prefix) => cmd.startsWith(prefix))
    ) {
      return true;
    }
    return false;
  };

  // Devuelve la configuración si esta llamada debe fallar; descuenta
  // `remaining` para poder programar fallos de una sola vez.
  const consumeFault = (cmd) => {
    const config = readFaults();
    if (!faultMatches(config, cmd)) return null;
    if (typeof config.remaining === "number") {
      if (config.remaining <= 0) return null;
      writeFaults({ ...config, remaining: config.remaining - 1 });
    }
    return config;
  };

  // Conteo de invocaciones por comando: permite a un spec esperar el reintento
  // automático de react-query sin depender de una pausa fija.
  const callsByCommand = new Map();
  let callsTotal = 0;

  // ---------- Validación de forma (e2e/ipc-shapes.json) ----------
  // Las formas se generan de src/bindings.ts (scripts/gen-ipc-shapes.mjs) y se
  // inyectan con e2e/mock.ts. Si el mock devuelve un campo con otro nombre (o
  // le falta uno requerido), el test falla aquí con la ruta exacta en vez de
  // pintar una pantalla vacía en la clínica.
  // Se lee en cada uso a propósito: el script con las formas se inyecta
  // después que el mock, así que al cargar este archivo aún no existe.
  const shapes = () => window.__E2E_IPC_SHAPES__ ?? null;
  // Violaciones acumuladas: el fixture de e2e/fixtures.ts las comprueba al
  // cerrar el test, de modo que un mock incoherente falla aunque la UI se
  // limite a pintar un estado de error.
  const shapeErrors = (window.__E2E_IPC_SHAPE_ERRORS__ = []);

  const recordShapeError = (message) => {
    if (shapeErrors.length < 50) shapeErrors.push(message);
  };

  const typeOfValue = (value) => {
    if (value === null) return "null";
    if (Array.isArray(value)) return "array";
    return typeof value;
  };

  const shapeLabel = (shape) => {
    switch (shape.kind) {
      case "ref":
        return shape.name;
      case "array":
        return `array<${shapeLabel(shape.items)}>`;
      case "union":
        return shape.of.map(shapeLabel).join(" | ");
      case "object":
      case "record":
        return "objeto";
      case "unknown":
        return "cualquiera";
      default:
        return shape.kind;
    }
  };

  const checkShape = (value, shape, path, issues, depth = 0) => {
    if (!shape || depth > 12 || shape.kind === "unknown") return;
    if (shape.kind === "ref") {
      const target = shapes()?.types?.[shape.name];
      if (target) checkShape(value, target, path, issues, depth + 1);
      return;
    }
    if (shape.kind === "union") {
      const fits = shape.of.some((option) => {
        const probe = [];
        checkShape(value, option, path, probe, depth + 1);
        return probe.length === 0;
      });
      if (!fits) {
        issues.push(
          `${path}: no coincide con ${shapeLabel(shape)} (llegó ${typeOfValue(value)})`,
        );
      }
      return;
    }
    if (shape.kind === "array") {
      if (!Array.isArray(value)) {
        issues.push(`${path}: se esperaba array, llegó ${typeOfValue(value)}`);
        return;
      }
      value.forEach((item, index) =>
        checkShape(item, shape.items, `${path}[${index}]`, issues, depth + 1),
      );
      return;
    }
    if (shape.kind === "record") {
      if (value === null || typeof value !== "object" || Array.isArray(value)) {
        issues.push(`${path}: se esperaba objeto, llegó ${typeOfValue(value)}`);
      }
      return;
    }
    if (shape.kind === "object") {
      if (value === null || typeof value !== "object" || Array.isArray(value)) {
        issues.push(`${path}: se esperaba objeto, llegó ${typeOfValue(value)}`);
        return;
      }
      for (const [name, field] of Object.entries(shape.fields)) {
        if (!(name in value)) {
          if (!field.optional) {
            issues.push(
              `${path}.${name}: falta el campo (así se llama en bindings.ts)`,
            );
          }
          continue;
        }
        checkShape(value[name], field.shape, `${path}.${name}`, issues, depth + 1);
      }
      return;
    }
    if (shape.kind === "null") {
      if (value !== null) {
        issues.push(`${path}: se esperaba null, llegó ${typeOfValue(value)}`);
      }
      return;
    }
    if (typeOfValue(value) !== shape.kind) {
      issues.push(
        `${path}: se esperaba ${shape.kind}, llegó ${typeOfValue(value)}`,
      );
    }
  };

  /** Valida la respuesta de un comando; lanza un error legible si no cuadra. */
  const assertResponseShape = (cmd, value) => {
    const spec = shapes()?.commands?.[cmd];
    if (!spec) return;
    const issues = [];
    checkShape(value, spec.result, `respuesta de ${cmd}`, issues);
    if (issues.length === 0) return;
    const message =
      `[e2e-ipc-shapes] La respuesta de "${cmd}" no coincide con src/bindings.ts:\n  ` +
      issues.join("\n  ") +
      "\nAjusta el handler en e2e/ipc-mock.script.js (o regenera las formas si cambió el tipo).";
    recordShapeError(message);
    throw new Error(message);
  };

  /** Valida que el payload tenga exactamente las claves de bindings.ts. */
  const assertArgsShape = (cmd, args) => {
    const spec = shapes()?.commands?.[cmd];
    if (!spec || spec.args.length === 0) return;
    const received = Object.keys(args ?? {}).sort();
    const expected = [...spec.args].sort();
    const missing = expected.filter((key) => !received.includes(key));
    const extra = received.filter((key) => !expected.includes(key));
    if (missing.length === 0 && extra.length === 0) return;
    const message =
      `[e2e-ipc-shapes] El payload de "${cmd}" no coincide con src/bindings.ts:` +
      (missing.length ? `\n  faltan: ${missing.join(", ")}` : "") +
      (extra.length ? `\n  sobran: ${extra.join(", ")}` : "") +
      `\n  esperado: [${expected.join(", ")}] · recibido: [${received.join(", ")}]`;
    recordShapeError(message);
    throw new Error(message);
  };

  // Ajustes de la clínica: la forma completa que espera bindings.ts
  // (ClinicSettings), con los campos de SMTP y marca en null por defecto.
  const clinicSettings = () => ({
    clinicName: "Clínica Veterinaria Central",
    clinicNit: "900000000-0",
    address: "Calle 12 # 34-56",
    phone: "3001234567",
    city: "Bogotá D.C.",
    logo: null,
    taxRate: 19,
    currency: "COP",
    signatureMode: "GRAPHIC",
    vetName: "Dra. Ana Pérez",
    vetLicense: "MVZ 12345",
    groqApiKey: null,
    certificate: null,
    pkcs12Password: null,
    smtpHost: null,
    smtpPort: null,
    smtpTls: null,
    smtpUsername: null,
    smtpPassword: null,
    smtpFrom: null,
    tagline: null,
    groomingDefaultDuration: 60,
    receptionTicketPrefix: null,
  });

  const handlers = {
    get_session: () => (sessionUser ? { ...sessionUser } : null),
    login: (args) => {
      const { username, password } = args.input ?? {};
      if (!username || !password)
        throw { type: "Validation", data: "Credenciales inválidas" };
      // Rol real del usuario: el menú y los botones se filtran por sus
      // capacidades, y los handlers de abajo rechazan lo que no le toca.
      const user = mockUsers.find((u) => u.username === username);
      if (!user || user.password !== password) {
        throw { type: "Validation", data: "Usuario o contraseña incorrectos" };
      }
      sessionUser = {
        id: user.id,
        username: user.username,
        fullName: user.fullName,
        role: user.role,
        mustChangePassword: false,
      };
      return { ...sessionUser };
    },
    logout: () => {
      sessionUser = null;
      return null;
    },
    db_health: () => ({
      ok: true,
      status: "connected",
      message: "ok",
      mode: "EMBEDDED",
      target: "C:/mock/isalab.fdb",
      host: "localhost",
      port: 3050,
      database: "C:/mock/isalab.fdb",
      user: "SYSDBA",
      clientFound: true,
      clientPath: "fbclient.dll",
      schemaVersion: 11,
      expectedSchemaVersion: 11,
      latencyMs: 2,
    }),
    get_dashboard_stats: () => {
      const list = samples.map(toListItem);
      const count = (st) => list.filter((s) => s.status === st).length;
      // Tendencia: últimos 7 días con el volumen de muestras recibidas.
      const weeklyVolume = [];
      for (let i = 6; i >= 0; i -= 1) {
        const d = new Date();
        d.setDate(d.getDate() - i);
        // Fecha LOCAL: con toISOString la tendencia se desfasa de noche.
        const key = localDate(d);
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
        // En Rust son f64 no opcionales (models/dashboard.rs): con cero
        // muestras el repositorio devuelve 0.0, nunca null.
        avgProcessingHours: 0,
        abnormalRate: 0,
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
      requireVetOrAdmin();
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
      requireVetOrAdmin();
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
      requireVetOrAdmin();
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
    list_analyzers: () => {
      requireVetOrAdmin();
      return analyzers.map((a) => ({
        ...a,
        rangeCount: referenceRanges.filter((r) => r.analyzerId === a.id).length,
      }));
    },
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
      requireVetOrAdmin();
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
        qualityIndex: null,
        qualityNote: null,
        qualitySeverity: null,
        rejectedAt: null,
        rejectedBy: null,
        rejectionReason: null,
        results: [],
      };
      nextSampleId += 1;
      samples.push(sample);
      return sample;
    },
    register_lab_result: (args) => {
      requireVetOrAdmin();
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
      requireVetOrAdmin();
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
      requireVetOrAdmin();
      const s = sampleById(args.sampleId);
      if (!s) throw { type: "NotFound", data: "Muestra no encontrada" };
      s.results = s.results.filter((r) => r.analyteId !== args.analyteId);
      return null;
    },
    set_sample_status: (args) => {
      requireVetOrAdmin();
      const s = sampleById(args.id);
      if (!s) throw { type: "NotFound", data: "Muestra no encontrada" };
      s.status = args.status;
      return { ...s, results: [...s.results] };
    },
    generate_sample_labels: (args) => {
      requireVetOrAdmin();
      const ids = args.sampleIds ?? [];
      for (const id of ids) {
        if (!sampleById(id))
          throw { type: "NotFound", data: "Muestra no encontrada" };
      }
      return {
        path: `C:/mock/etiquetas-${ids.join("-")}.pdf`,
        fileName: `ISALAB_Etiquetas_${ids.join("-")}.pdf`,
        sampleCode: `M-${currentYear()}-0001`,
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
            subtitle: `${s.sampleTypeName} · ${s.status}`,
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
    get_clinic_settings: () => clinicSettings(),
    save_clinic_settings: (args) => {
      requireAdmin();
      // Como el backend: devuelve los ajustes guardados completos.
      return { ...clinicSettings(), ...args.input };
    },
    import_clinic_logo: (args) => {
      requireAdmin();
      return args.sourcePath;
    },
    list_secondary_logos: () => [
      {
        id: 1,
        name: "Logo Falso",
        mimeType: "image/png",
        sizeBytes: 18432,
        createdAt: daysAgo(20, 10, 0),
      },
      {
        id: 2,
        name: "Proyecto X",
        mimeType: "image/png",
        sizeBytes: 9216,
        createdAt: daysAgo(19, 11, 0),
      },
    ],
    import_secondary_logo: (args) => {
      requireAdmin();
      return {
        id: 3,
        name: args.input.name,
        mimeType: "image/png",
        sizeBytes: 12288,
        createdAt: daysAgo(18, 12, 0),
      };
    },
    delete_secondary_logo: () => {
      requireAdmin();
      return null;
    },
    // ----- Catálogos y páginas del resto de la app (estados vacíos) -------
    // Necesarios para el test de viewport 1366x768, que navega por TODAS las
    // vistas: cada página debe montar sin datos y sin excepciones.
    list_species: () => species,
    list_breeds: () => [
      { id: 1, speciesId: 1, name: "Labrador" },
      { id: 2, speciesId: 1, name: "Criollo" },
    ],
    list_owners: () => owners,
    list_users: () => {
      requireAdmin();
      return mockUsers.map((u, idx) => ({
        id: u.id,
        username: u.username,
        fullName: u.fullName,
        role: u.role,
        active: true,
        mustChangePassword: false,
        createdAt: daysAgo(30 - idx, 8, 0),
      }));
    },
    list_audit_log: (args = {}) => {
      requireAdmin();
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
    // ----- Vistas sin datos modelados (los E2E solo verifican que montan) --
    // Citas, peluquería, estancias, recursos, inventario y marca: el mock
    // devuelve colecciones vacías para que las páginas pinten su estado vacío
    // en vez de un error. Si algún spec empieza a ejercitarlas, se modela el
    // estado aquí (no en la allowlist).
    list_appointments: (args) => {
      let rows = agenda.appointments;
      if (args.date) {
        rows = rows.filter((a) => a.scheduledAt.slice(0, 10) === args.date);
      }
      if (args.status) rows = rows.filter((a) => a.status === args.status);
      // Las citas del mock se crean sin profesional; el filtro sigue siendo real.
      if (args.professionalId != null) {
        rows = rows.filter((a) => a.professionalId === args.professionalId);
      }
      return rows.map(toAppointmentListItem);
    },
    list_grooming_day: () => [],
    list_grooming_reminders: () => [],
    list_stays: () => [],
    list_resources: () => [],
    list_resource_board: () => [],
    list_products: () => [],
    list_stock_movements: () => [],
    get_clinic_branding: () => ({
      clinicName: "Clínica Veterinaria Central",
      tagline: null,
    }),
    get_clinic_logo_content: () => null,
    get_secondary_logo_content: () => null,

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
      requireVetOrAdmin();
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
      requireVetOrAdmin();
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
    register_payment: (args) => {
      requireBill();
      return registerPayment(args.input ?? {});
    },
    void_payment: (args) => {
      requireBill();
      return voidPayment(args.paymentId, args.reason);
    },
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
      requireVetOrAdmin();
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
      requireBill();
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
      requireBill();
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
      // El backend devuelve Result<CashSessionSummary>: sin caja abierta falla
      // con Validation, no con null (bindings.ts no lo declara nullable).
      const session = args.sessionId == null ? openSession() : resolveSession(args.sessionId);
      if (!session) {
        throw {
          type: "Validation",
          data: "No hay una caja abierta. Abre la caja primero.",
        };
      }
      return buildSummary(session);
    },
    close_cash_session: (args) => {
      requireBill();
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
    list_all_service_categories: () => {
      requireAdmin();
      return serviceCategories;
    },
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
    /** Check-in de una cita: crea su visita (con el servicio de la cita) y la
     * deja ATENDIDA, como el backend. */
    check_in_appointment: (args) => {
      requireReceive();
      const appt = appointmentById(args.id);
      if (!appt) throw { type: "NotFound", data: "Cita no encontrada" };
      if (appt.status !== "PROGRAMADA" && appt.status !== "CONFIRMADA") {
        throw { type: "Validation", data: "La cita ya no admite check-in" };
      }
      // Idempotente como el backend: si la cita ya tiene visita, se devuelve en
      // vez de duplicarla.
      const existing = money.visits.find((v) => v.appointmentId === appt.id);
      if (existing) return toVisit(existing);
      const id = money.nextVisitId;
      money.nextVisitId += 1;
      const visit = {
        id,
        code: `V-${currentYear()}-${String(id).padStart(4, "0")}`,
        patientId: appt.patientId,
        patientName: appt.patientName,
        ownerId: appt.ownerId,
        ownerName: appt.ownerName,
        appointmentId: appt.id,
        arrivedAt: stamp(),
        priority: "NORMAL",
        status: "EN_ESPERA",
        notes: null,
        services: [],
      };
      const service = services.find((s) => s.id === appt.serviceId);
      if (service) {
        visit.services.push(
          visitLine(visit, service, { professionalId: appt.professionalId }),
        );
      }
      money.visits.push(visit);
      appt.status = "ATENDIDA";
      appt.updatedAt = stamp();
      return toVisit(visit);
    },
    /** Programa una cita desde el diálogo de la agenda. */
    create_appointment: (args) => {
      requireReceive();
      const { input } = args;
      const patient = patientById(input.patientId);
      const service = serviceById(input.serviceId);
      if (!patient || !service) {
        throw { type: "Validation", data: "Paciente o servicio inválido" };
      }
      const id = agenda.nextId;
      agenda.nextId += 1;
      const appt = buildAppointment({
        id,
        patient,
        service,
        scheduledAt: asStamp(input.scheduledAt),
        durationMin: input.durationMin,
        professionalId: input.professionalId,
        notes: input.notes,
        status: "PROGRAMADA",
        createdAt: stamp(),
      });
      agenda.appointments.push(appt);
      return { ...appt };
    },
    /** Cambia el estado de una cita (máquina de estados del backend). */
    set_appointment_status: (args) => {
      requireReceive();
      const appt = appointmentById(args.id);
      if (!appt) throw { type: "NotFound", data: "Cita no encontrada" };
      return moveAppointment(appt, args.input?.status, args.input?.notes);
    },
    /** Anula una cita: es la transición a CANCELADA. */
    cancel_appointment: (args) => {
      requireReceive();
      const appt = appointmentById(args.id);
      if (!appt) throw { type: "NotFound", data: "Cita no encontrada" };
      return moveAppointment(appt, "CANCELADA", null);
    },
    /** Historial clínico: ficha, propietario, actividad y cobros del paciente. */
    get_clinical_history: (args) => {
      const patient = patientById(args.patientId);
      if (!patient) throw { type: "NotFound", data: "Paciente no encontrado" };
      return {
        patient,
        owner: owners.find((o) => o.id === patient.ownerId) ?? null,
        consultations: [],
        vaccines: [],
        samples: [],
        // Visitas reales (las crea el check-in) + las sembradas para el historial.
        visits: [
          ...money.visits
            .filter((v) => v.patientId === patient.id)
            .map(toVisitListItem),
          ...clinicalFixtures.visits.filter((v) => v.patientId === patient.id),
        ].sort((a, b) => b.arrivedAt.localeCompare(a.arrivedAt)),
        groomingSessions: clinicalFixtures.groomingSessions.filter(
          (g) => g.patientId === patient.id,
        ),
        charges: [],
      };
    },
    /** Órdenes de laboratorio del paciente que pinta la ficha del historial. */
    list_patient_lab_orders: () => [],
    create_visit: (args) => {
      requireReceive();
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
        code: `V-${currentYear()}-${String(id).padStart(4, "0")}`,
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
      requireReceive();
      const visit = visitById(args.id);
      if (!visit) throw { type: "NotFound", data: "Visita no encontrada" };
      if (visit.status === "FACTURADA" && args.status !== "FACTURADA") {
        throw { type: "Validation", data: "Una visita facturada no cambia de estado" };
      }
      visit.status = args.status;
      return toVisit(visit);
    },
    add_visit_service: (args) => {
      requireReceive();
      const visit = visitById(args.visitId);
      if (!visit) throw { type: "NotFound", data: "Visita no encontrada" };
      const service = services.find((s) => s.id === args.input?.serviceId);
      if (!service) throw { type: "Validation", data: "Servicio inválido" };
      const line = visitLine(visit, service, args.input ?? {});
      visit.services.push(line);
      return { ...line };
    },
    remove_visit_service: (args) => {
      requireReceive();
      const visit = visitById(args.visitId);
      if (!visit) throw { type: "NotFound", data: "Visita no encontrada" };
      visit.services = visit.services.filter((l) => l.id !== args.lineId);
      return null;
    },
    update_visit_service_status: (args) => {
      requireReceive();
      const visit = visitById(args.visitId);
      const line = visit?.services.find((l) => l.id === args.lineId);
      if (!visit || !line) throw { type: "NotFound", data: "Línea no encontrada" };
      line.status = args.input?.status ?? line.status;
      if (line.status === "EN_PROCESO" && !line.startedAt) line.startedAt = stamp();
      if (line.status === "COMPLETADA" && !line.finishedAt) line.finishedAt = stamp();
      return { ...line };
    },
    checkout_visit: (args) => {
      requireBill();
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
    get_qc_chart: () => null,
    list_reports: () => [],
    // Diagnóstico: la UI reenvía errores al log de Rust y puede exportarlo.
    log_frontend: () => null,
    export_diagnostics: () => "C:/mock/isalab-diagnostico.zip",
    list_analyzer_sources: () => {
      requireVetOrAdmin();
      return [];
    },
    list_analyzer_import_jobs: () => {
      requireVetOrAdmin();
      return [];
    },
    list_failed_analyzer_imports: () => {
      requireVetOrAdmin();
      return [];
    },
    poll_analyzer_source: () => {
      requireVetOrAdmin();
      return null;
    },
    // Listeners/emits del runtime Tauri (Firebird events, app-ready, …)
    "plugin:event|listen": () => () => {},
    "plugin:event|unlisten": () => null,
    "plugin:event|emit": () => null,
  };

  // Error legible para un comando sin handler: el mensaje nombra el comando,
  // los argumentos y los dos remedios posibles. Se rechaza con un `Error` real
  // (no con un AppError serializado) porque `typedError` en src/bindings.ts
  // relanza los Error y convierte el resto en `{ status: "error" }`, que las
  // páginas suelen pintar como estado vacío sin romper el test.
  const missingHandlerError = (cmd, args) => {
    let detail;
    try {
      detail = JSON.stringify(args) ?? "undefined";
    } catch {
      detail = "<argumentos no serializables>";
    }
    return new Error(
      `[e2e-ipc-mock] Comando sin handler: "${cmd}" (args: ${detail}). ` +
        "Añade el handler en e2e/ipc-mock.script.js o declara el comando en " +
        "e2e/ipc-mock.allowlist.mjs si los E2E no deben ejercitarlo.",
    );
  };

  const invoke = (cmd, args = {}) => {
    callsTotal += 1;
    callsByCommand.set(cmd, (callsByCommand.get(cmd) ?? 0) + 1);

    const fault = consumeFault(cmd);
    if (fault) {
      const error = {
        type: fault.errorType ?? "Database",
        data: fault.message ?? DB_DOWN_MESSAGE,
      };
      console.error(`[e2e-ipc-mock] fallo inyectado en "${cmd}": ${error.data}`);
      return Promise.reject(error);
    }

    const fn = handlers[cmd];
    if (!fn) {
      const error = missingHandlerError(cmd, args);
      console.error(error.message);
      return Promise.reject(error);
    }
    try {
      const value = fn(args);
      // El payload se valida solo si el comando lo aceptó: un rechazo por rol
      // (Forbidden) ocurre antes de mirar los argumentos, como en Rust.
      assertArgsShape(cmd, args);
      assertResponseShape(cmd, value);
      return Promise.resolve(value);
    } catch (err) {
      console.error(err.message ?? String(err));
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
  // API de control de fallos para los specs (ver cabecera del archivo).
  window.__E2E_IPC_FAULTS__ = {
    key: FAULTS_KEY,
    groups: () => Object.keys(FAULT_GROUPS),
    get: readFaults,
    set: (config) => {
      writeFaults(config);
      return readFaults();
    },
    clear: () => {
      writeFaults({});
      return readFaults();
    },
    stats: () => ({
      total: callsTotal,
      byCommand: Object.fromEntries(callsByCommand),
    }),
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
