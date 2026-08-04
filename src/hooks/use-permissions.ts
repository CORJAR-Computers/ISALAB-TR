import { useSessionStore } from "@/stores/session-store";

export function usePermissions() {
  const session = useSessionStore((s) => s.session);
  const role = session?.role || "GUEST";

  return {
    role,
    isAdmin: role === "ADMIN",
    isVet: role === "VETERINARIO",
    isAuxiliar: role === "AUXILIAR",
    isVetOrAdmin: role === "ADMIN" || role === "VETERINARIO",
  };
}
