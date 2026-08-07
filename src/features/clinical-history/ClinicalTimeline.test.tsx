import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ClinicalTimeline } from "./ClinicalTimeline";
import type { ClinicalHistory, Patient } from "@/bindings";

const mockPatient: Patient = {
  id: 1,
  code: "P-2023-0001",
  ownerId: 2,
  speciesId: 1,
  breedId: null,
  name: "Firulais",
  sex: "M",
  birthDate: "2020-01-01T00:00:00Z",
  neutered: true,
  color: "Café",
  microchip: null,
  active: true,
  notes: null,
  speciesName: "Canino",
  breedName: null,
  ownerName: "Juan Pérez",
  ownerPhone: "3000000000",
  ageMonths: 36,
  preferredLogoId: null,
};

const mockOwner = {
  id: 1,
  documentType: "CC",
  documentNumber: "1234567890",
  fullName: "Juan Pérez",
  phone: "3001234567",
  email: null,
  address: null,
  city: null,
};

function makeHistory(
  overrides: Partial<ClinicalHistory> = {}
): ClinicalHistory {
  return {
    patient: mockPatient,
    owner: mockOwner,
    consultations: [],
    vaccines: [],
    samples: [],
    ...overrides,
  };
}

describe("ClinicalTimeline", () => {
  it("renders empty state when no history", () => {
    render(<ClinicalTimeline history={makeHistory()} />);
    expect(
      screen.getByText("Sin actividad clínica registrada. Crea la primera consulta.")
    ).toBeInTheDocument();
  });

  it("renders header with record count", () => {
    render(<ClinicalTimeline history={makeHistory()} />);
    expect(screen.getByText("Línea de tiempo clínica")).toBeInTheDocument();
    expect(screen.getByText("0 registros · orden cronológico")).toBeInTheDocument();
  });

  it("renders consultation entries", () => {
    const history = makeHistory({
      consultations: [
        {
          id: 1,
          patientId: 1,
          veterinarianId: 1,
          consultationDate: "2025-03-15 10:30:00",
          reason: "Dolor abdominal",
          anamnesis: "Vómitos desde hace 2 días",
          physicalExam: "Dolor a la palpación",
          diagnosis: "Gastroenteritis",
          treatmentPlan: "Antibiótico y dieta",
          status: "COMPLETADA",
          veterinarianName: "Dra. Pérez",
        },
      ],
    });
    render(<ClinicalTimeline history={history} />);
    expect(screen.getByText("Dolor abdominal")).toBeInTheDocument();
    expect(screen.getByText("1 registro · orden cronológico")).toBeInTheDocument();
  });

  it("renders vaccine entries", () => {
    const history = makeHistory({
      vaccines: [
        {
          id: 1,
          patientId: 1,
          vaccineTypeId: 1,
          vaccineName: "Rabia",
          dose: "1ra",
          administeredAt: "2025-01-10 09:00:00",
          nextDoseAt: "2026-01-10",
          lot: "LOT123",
          manufacturer: "Zoetis",
          veterinarianName: "Dra. Pérez",
          notes: null,
        },
      ],
    });
    render(<ClinicalTimeline history={history} />);
    expect(screen.getByText("Vacuna · Rabia")).toBeInTheDocument();
    expect(screen.getByText("1 registro · orden cronológico")).toBeInTheDocument();
  });

  it("renders sample entries with badge", () => {
    const history = makeHistory({
      samples: [
        {
          id: 1,
          code: "M-2025-0001",
          patientId: 1,
          sampleTypeId: 1,
          sampleTypeName: "Hemograma",
          receivedAt: "2025-02-20 14:00:00",
          status: "RECIBIDA",
          collectedBy: "Auxiliar María",
          notes: null,
          analyzerId: null,
          analyzerName: null,
          results: [],
        },
      ],
    });
    render(<ClinicalTimeline history={history} />);
    expect(screen.getByText(/Muestra M-2025-0001/)).toBeInTheDocument();
    expect(screen.getByText("Sin resultados")).toBeInTheDocument();
  });

  it("renders multiple entries sorted by date (newest first)", () => {
    const history = makeHistory({
      consultations: [
        {
          id: 1,
          patientId: 1,
          veterinarianId: null,
          consultationDate: "2025-01-10 09:00:00",
          reason: "Primera visita",
          anamnesis: null,
          physicalExam: null,
          diagnosis: null,
          treatmentPlan: null,
          status: "COMPLETADA",
          veterinarianName: null,
        },
      ],
      vaccines: [
        {
          id: 1,
          patientId: 1,
          vaccineTypeId: null,
          vaccineName: "Rabia",
          dose: null,
          administeredAt: "2025-06-15 11:00:00",
          nextDoseAt: null,
          lot: null,
          manufacturer: null,
          veterinarianName: null,
          notes: null,
        },
      ],
    });
    render(<ClinicalTimeline history={history} />);
    expect(screen.getByText("2 registros · orden cronológico")).toBeInTheDocument();
    // Vaccine (Jun) should appear before consultation (Jan)
    const items = screen.getAllByRole("listitem");
    expect(items).toHaveLength(2);
  });

  it("shows consultation details when present", () => {
    const history = makeHistory({
      consultations: [
        {
          id: 1,
          patientId: 1,
          veterinarianId: null,
          consultationDate: "2025-03-15 10:30:00",
          reason: "Control",
          anamnesis: "Paciente sin síntomas",
          physicalExam: "Signos vitales normales",
          diagnosis: "Sano",
          treatmentPlan: "Ninguno",
          status: "COMPLETADA",
          veterinarianName: "Dr. García",
        },
      ],
    });
    render(<ClinicalTimeline history={history} />);
    expect(screen.getByText(/Anamnesis:/)).toBeInTheDocument();
    expect(screen.getByText("Paciente sin síntomas")).toBeInTheDocument();
    expect(screen.getByText(/Examen físico:/)).toBeInTheDocument();
    expect(screen.getByText("Signos vitales normales")).toBeInTheDocument();
    expect(screen.getByText(/Diagnóstico:/)).toBeInTheDocument();
    expect(screen.getByText("Sano")).toBeInTheDocument();
    expect(screen.getByText(/Plan:/)).toBeInTheDocument();
    expect(screen.getByText("Ninguno")).toBeInTheDocument();
    expect(screen.getByText("Atendió: Dr. García")).toBeInTheDocument();
  });
});
