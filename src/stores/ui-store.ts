import { create } from "zustand";

export type Theme = "light" | "dark";

export type View =
  | "dashboard"
  | "agenda"
  | "patients"
  | "clinical-history"
  | "samples"
  | "worklist"
  | "surgeries"
  | "vaccines"
  | "invoices"
  | "reports"
  | "users"
  | "audit-log"
  | "settings";

type UiState = {
  theme: Theme;
  sidebarOpen: boolean;
  view: View;
  /** Paciente activo seleccionado (para el historial clínico). */
  activePatientId: number | null;
  /** Solicitud global para abrir el diálogo de nuevo paciente (p. ej. desde el dashboard). */
  newPatientRequest: number;
  /** Control del diálogo Acerca de (CORJAR Computers Solutions). */
  aboutOpen: boolean;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  setSidebarOpen: (open: boolean) => void;
  navigate: (view: View) => void;
  setActivePatient: (id: number | null) => void;
  requestNewPatient: () => void;
  consumeNewPatientRequest: () => void;
  setAboutOpen: (open: boolean) => void;
};

const storedTheme = (): Theme => {
  try {
    const t = localStorage.getItem("isalab-theme");
    return t === "dark" ? "dark" : "light";
  } catch {
    return "light";
  }
};

export const useUiStore = create<UiState>((set) => ({
  theme: storedTheme(),
  sidebarOpen: true,
  view: "dashboard",
  activePatientId: null,
  newPatientRequest: 0,
  aboutOpen: false,

  setTheme: (theme) => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    try {
      localStorage.setItem("isalab-theme", theme);
    } catch {
      /* ignore */
    }
    set({ theme });
  },

  toggleTheme: () =>
    set((s) => {
      const next: Theme = s.theme === "light" ? "dark" : "light";
      document.documentElement.classList.toggle("dark", next === "dark");
      try {
        localStorage.setItem("isalab-theme", next);
      } catch {
        /* ignore */
      }
      return { theme: next };
    }),

  setSidebarOpen: (open) => set({ sidebarOpen: open }),
  navigate: (view) => set({ view }),
  setActivePatient: (id) => set({ activePatientId: id }),
  requestNewPatient: () => set((s) => ({ newPatientRequest: s.newPatientRequest + 1 })),
  consumeNewPatientRequest: () => set({ newPatientRequest: 0 }),
  setAboutOpen: (open) => set({ aboutOpen: open }),
}));
