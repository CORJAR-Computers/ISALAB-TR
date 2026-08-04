import { create } from "zustand";
import type { SessionUser } from "@/bindings";
import { api } from "@/lib/api";

type SessionState = {
  /** Usuario autenticado (null = pantalla de login). */
  session: SessionUser | null;
  /** `true` tras consultar la sesión al backend (evita parpadeo de login). */
  hydrated: boolean;
  /** Controla la apertura del diálogo de cambio de contraseña. */
  changePasswordOpen: boolean;
  setSession: (user: SessionUser | null) => void;
  setHydrated: (v: boolean) => void;
  openChangePassword: () => void;
  closeChangePassword: () => void;
  logout: () => Promise<void>;
};

export const useSessionStore = create<SessionState>((set) => ({
  session: null,
  hydrated: false,
  changePasswordOpen: false,
  setSession: (session) => set({ session }),
  setHydrated: (hydrated) => set({ hydrated }),
  openChangePassword: () => set({ changePasswordOpen: true }),
  closeChangePassword: () => set({ changePasswordOpen: false }),
  logout: async () => {
    try {
      await api.logout();
    } catch (e) {
      console.error("Error al cerrar sesión", e);
    } finally {
      set({ session: null });
    }
  },
}));