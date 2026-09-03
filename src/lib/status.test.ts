import { describe, it, expect } from "vitest";
import {
  SAMPLE_STATUS,
  RESULT_STATUS,
  CONSULTATION_STATUS,
  SEX_LABEL,
  ROLE_LABEL,
  SURGERY_STATUS,
  INVOICE_STATUS,
  PAYMENT_METHOD_LABEL,
  ANESTHESIA_OPTIONS,
} from "./status";

describe("SAMPLE_STATUS", () => {
  it("has all expected statuses", () => {
    expect(Object.keys(SAMPLE_STATUS)).toEqual([
      "RECIBIDA",
      "EN_PROCESO",
      "FINALIZADA",
      "ANULADA",
      "RECHAZADA",
    ]);
  });

  it("RECIBIDA has secondary variant", () => {
    expect(SAMPLE_STATUS.RECIBIDA.variant).toBe("secondary");
    expect(SAMPLE_STATUS.RECIBIDA.label).toBe("Recibida");
  });

  it("EN_PROCESO has warning variant", () => {
    expect(SAMPLE_STATUS.EN_PROCESO.variant).toBe("warning");
  });

  it("FINALIZADA has success variant", () => {
    expect(SAMPLE_STATUS.FINALIZADA.variant).toBe("success");
  });

  it("ANULADA has destructive variant", () => {
    expect(SAMPLE_STATUS.ANULADA.variant).toBe("destructive");
  });

  it("RECHAZADA has destructive variant", () => {
    expect(SAMPLE_STATUS.RECHAZADA.variant).toBe("destructive");
    expect(SAMPLE_STATUS.RECHAZADA.label).toBe("Rechazada");
  });
});

describe("RESULT_STATUS", () => {
  it("has all expected statuses", () => {
    expect(Object.keys(RESULT_STATUS)).toEqual([
      "NORMAL",
      "ALTO",
      "BAJO",
      "SIN_RANGO",
      "CRITICO_ALTO",
      "CRITICO_BAJO",
    ]);
  });

  it("NORMAL has success variant", () => {
    expect(RESULT_STATUS.NORMAL.variant).toBe("success");
    expect(RESULT_STATUS.NORMAL.label).toBe("Normal");
  });

  it("ALTO has warning variant", () => {
    expect(RESULT_STATUS.ALTO.variant).toBe("warning");
    expect(RESULT_STATUS.ALTO.label).toContain("Alto");
  });

  it("BAJO has destructive variant", () => {
    expect(RESULT_STATUS.BAJO.variant).toBe("destructive");
    expect(RESULT_STATUS.BAJO.label).toContain("Bajo");
  });

  it("SIN_RANGO has outline variant", () => {
    expect(RESULT_STATUS.SIN_RANGO.variant).toBe("outline");
  });

  it("CRITICO_ALTO has destructive pulse variant", () => {
    expect(RESULT_STATUS.CRITICO_ALTO.variant).toBe("destructive");
    expect(RESULT_STATUS.CRITICO_ALTO.className).toContain("animate-pulse");
  });

  it("CRITICO_BAJO has destructive pulse variant", () => {
    expect(RESULT_STATUS.CRITICO_BAJO.variant).toBe("destructive");
  });
});

describe("CONSULTATION_STATUS", () => {
  it("has all expected statuses", () => {
    expect(Object.keys(CONSULTATION_STATUS)).toEqual([
      "COMPLETADA",
      "PENDIENTE",
      "CANCELADA",
    ]);
  });

  it("COMPLETADA has success variant", () => {
    expect(CONSULTATION_STATUS.COMPLETADA.variant).toBe("success");
  });

  it("PENDIENTE has warning variant", () => {
    expect(CONSULTATION_STATUS.PENDIENTE.variant).toBe("warning");
  });

  it("CANCELADA has destructive variant", () => {
    expect(CONSULTATION_STATUS.CANCELADA.variant).toBe("destructive");
  });
});

describe("SEX_LABEL", () => {
  it("maps M to Macho", () => {
    expect(SEX_LABEL.M).toBe("Macho");
  });

  it("maps F to Hembra", () => {
    expect(SEX_LABEL.F).toBe("Hembra");
  });
});

describe("ROLE_LABEL", () => {
  it("maps all roles", () => {
    expect(ROLE_LABEL.ADMIN).toBe("Administrador");
    expect(ROLE_LABEL.VETERINARIO).toBe("Veterinario");
    expect(ROLE_LABEL.AUXILIAR).toBe("Auxiliar");
  });
});

describe("SURGERY_STATUS", () => {
  it("has all expected statuses", () => {
    expect(Object.keys(SURGERY_STATUS)).toEqual([
      "PROGRAMADA",
      "EN_CURSO",
      "COMPLETADA",
      "CANCELADA",
    ]);
  });

  it("EN_CURSO has warning variant", () => {
    expect(SURGERY_STATUS.EN_CURSO.variant).toBe("warning");
  });
});

describe("INVOICE_STATUS", () => {
  it("has all expected statuses", () => {
    expect(Object.keys(INVOICE_STATUS)).toEqual([
      "EMITIDA",
      "PAGADA",
      "ANULADA",
    ]);
  });

  it("EMITIDA has warning variant", () => {
    expect(INVOICE_STATUS.EMITIDA.variant).toBe("warning");
  });

  it("PAGADA has success variant", () => {
    expect(INVOICE_STATUS.PAGADA.variant).toBe("success");
  });
});

describe("PAYMENT_METHOD_LABEL", () => {
  it("maps all payment methods", () => {
    expect(PAYMENT_METHOD_LABEL.EFECTIVO).toBe("Efectivo");
    expect(PAYMENT_METHOD_LABEL.TRANSFERENCIA).toBe("Transferencia");
    expect(PAYMENT_METHOD_LABEL.TARJETA_CREDITO).toBe("Tarjeta crédito");
    expect(PAYMENT_METHOD_LABEL.TARJETA_DEBITO).toBe("Tarjeta débito");
  });
});

describe("ANESTHESIA_OPTIONS", () => {
  it("contains 5 options", () => {
    expect(ANESTHESIA_OPTIONS).toHaveLength(5);
  });

  it("includes General inhalatoria", () => {
    expect(ANESTHESIA_OPTIONS).toContain("General inhalatoria");
  });

  it("includes Sin anestesia", () => {
    expect(ANESTHESIA_OPTIONS).toContain("Sin anestesia");
  });
});
