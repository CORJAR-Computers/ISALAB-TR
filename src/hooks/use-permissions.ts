import { useSessionStore } from "@/stores/session-store";

/**
 * Capacidades del rol activo, espejo del mapa de Rust (auth.rs
 * `capabilities_for_role`). Los comandos exigen capacidades, no roles
 * literales; la UI usa la misma lógica para mostrar/ocultar.
 */
export type Capabilities = {
  role: string;
  isVet: boolean;
  /** Check-in de visitas, cola de espera, servicios de la visita. */
  canReceive: boolean;
  /** Registro clínico (consultas, resultados, cirugías, vacunas). */
  canClinical: boolean;
  /** Fichas y sesiones de peluquería. */
  canGroom: boolean;
  /** Facturación y cobro de visitas. */
  canBill: boolean;
  isAdmin: boolean;
  isVetOrAdmin: boolean;
};

const byRole = (role: string): Capabilities => {
  switch (role) {
    case "ADMIN":
      return {
        role,
        isVet: false,
        canReceive: true,
        canClinical: true,
        canGroom: true,
        canBill: true,
        isAdmin: true,
        isVetOrAdmin: true,
      };
    case "VETERINARIO":
      return {
        role,
        isVet: true,
        canReceive: true,
        canClinical: true,
        canGroom: false,
        canBill: true,
        isAdmin: false,
        isVetOrAdmin: true,
      };
    case "AUXILIAR":
      return {
        role,
        isVet: false,
        canReceive: true,
        canClinical: false,
        canGroom: true,
        canBill: true,
        isAdmin: false,
        isVetOrAdmin: false,
      };
    case "RECEPCION":
      return {
        role,
        isVet: false,
        canReceive: true,
        canClinical: false,
        canGroom: false,
        canBill: true,
        isAdmin: false,
        isVetOrAdmin: false,
      };
    case "PELUQUERO":
      return {
        role,
        isVet: false,
        canReceive: false,
        canClinical: false,
        canGroom: true,
        canBill: false,
        isAdmin: false,
        isVetOrAdmin: false,
      };
    default:
      return {
        role,
        isVet: false,
        canReceive: false,
        canClinical: false,
        canGroom: false,
        canBill: false,
        isAdmin: false,
        isVetOrAdmin: false,
      };
  }
};

export function usePermissions(): Capabilities {
  const session = useSessionStore((s) => s.session);
  return byRole(session?.role || "GUEST");
}
