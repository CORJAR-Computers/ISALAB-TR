import type { VariantProps } from "class-variance-authority";
import type { badgeVariants } from "@/components/ui/badge";

type BadgeVariant = NonNullable<
  VariantProps<typeof badgeVariants>["variant"]
>;

export const SAMPLE_STATUS: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  RECIBIDA: { label: "Recibida", variant: "secondary" },
  EN_PROCESO: { label: "En proceso", variant: "warning" },
  FINALIZADA: { label: "Finalizada", variant: "success" },
  ANULADA: { label: "Anulada", variant: "destructive" },
  RECHAZADA: { label: "Rechazada", variant: "destructive" },
};

export const RESULT_STATUS: Record<
  string,
  { label: string; variant: BadgeVariant; className?: string }
> = {
  NORMAL: { label: "Normal", variant: "success" },
  ALTO: { label: "Alto ↑", variant: "warning" },
  BAJO: { label: "Bajo ↓", variant: "destructive" },
  SIN_RANGO: { label: "Sin rango", variant: "outline" },
  CRITICO_ALTO: {
    label: "Crítico alto ⚠",
    variant: "destructive",
    className: "animate-pulse",
  },
  CRITICO_BAJO: {
    label: "Crítico bajo ⚠",
    variant: "destructive",
    className: "animate-pulse",
  },
};

/** Interferencias preanalíticas registradas en la recepción de la muestra. */
export const QUALITY_INDEX_LABEL: Record<string, string> = {
  NORMAL: "Sin interferencia",
  HEMOLISIS: "Hemólisis",
  LIPEMIA: "Lipemia",
  ICTERICIA: "Ictericia",
  COAGULO: "Coágulo",
  INSUFICIENTE: "Volumen insuficiente",
  CONTAMINADA: "Contaminada",
};

export const QUALITY_SEVERITY_LABEL: Record<string, string> = {
  LEVE: "Leve",
  MODERADA: "Moderada",
  MARCADA: "Marcada",
};

export const CONSULTATION_STATUS: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  COMPLETADA: { label: "Completada", variant: "success" },
  PENDIENTE: { label: "Pendiente", variant: "warning" },
  CANCELADA: { label: "Cancelada", variant: "destructive" },
};

export const SEX_LABEL: Record<string, string> = {
  M: "Macho",
  F: "Hembra",
};

export const ROLE_LABEL: Record<string, string> = {
  ADMIN: "Administrador",
  VETERINARIO: "Veterinario",
  AUXILIAR: "Auxiliar",
};

export const SURGERY_STATUS: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  PROGRAMADA: { label: "Programada", variant: "secondary" },
  EN_CURSO: { label: "En curso", variant: "warning" },
  COMPLETADA: { label: "Completada", variant: "success" },
  CANCELADA: { label: "Cancelada", variant: "destructive" },
};

export const INVOICE_STATUS: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  EMITIDA: { label: "Emitida", variant: "warning" },
  PAGADA: { label: "Pagada", variant: "success" },
  ANULADA: { label: "Anulada", variant: "destructive" },
};

export const PAYMENT_METHOD_LABEL: Record<string, string> = {
  EFECTIVO: "Efectivo",
  TRANSFERENCIA: "Transferencia",
  TARJETA_CREDITO: "Tarjeta crédito",
  TARJETA_DEBITO: "Tarjeta débito",
};

export const LAB_ORDER_STATUS: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  RECIBIDA: { label: "Recibida", variant: "secondary" },
  EN_PROCESO: { label: "En proceso", variant: "warning" },
  COMPLETADA: { label: "Completada", variant: "success" },
  ANULADA: { label: "Anulada", variant: "destructive" },
};

export const LAB_ORDER_PRIORITY: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  NORMAL: { label: "Normal", variant: "outline" },
  URGENTE: { label: "Urgente", variant: "destructive" },
};

/**
 * Transiciones de estado permitidas para una orden de laboratorio
 * (validadas por el backend; aqui solo se usan para habilitar acciones).
 */
export const LAB_ORDER_STATUS_TRANSITIONS: Record<string, string[]> = {
  RECIBIDA: ["EN_PROCESO", "COMPLETADA", "ANULADA"],
  EN_PROCESO: ["COMPLETADA", "ANULADA"],
  COMPLETADA: [],
  ANULADA: [],
};

export const ANESTHESIA_OPTIONS = [
  "General inhalatoria",
  "General inyectable",
  "Local / regional",
  "Sedación + local",
  "Sin anestesia",
] as const;
