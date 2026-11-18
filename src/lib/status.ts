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

/** Estados de una cita de la agenda generalizada (migración 0028). */
export const APPOINTMENT_STATUS: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  PROGRAMADA: { label: "Programada", variant: "secondary" },
  CONFIRMADA: { label: "Confirmada", variant: "default" },
  EN_ATENCION: { label: "En atención", variant: "warning" },
  ATENDIDA: { label: "Atendida", variant: "success" },
  NO_ASISTIO: { label: "No asistió", variant: "destructive" },
  CANCELADA: { label: "Cancelada", variant: "outline" },
};

/** Estados de una visita de recepción (migración 0027). */
export const VISIT_STATUS: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  EN_ESPERA: { label: "En espera", variant: "secondary" },
  EN_ATENCION: { label: "En atención", variant: "warning" },
  FINALIZADA: { label: "Finalizada", variant: "success" },
  FACTURADA: { label: "Facturada", variant: "default" },
  CANCELADA: { label: "Cancelada", variant: "destructive" },
};

/** Estados de una línea de servicio de visita. */
export const VISIT_SERVICE_STATUS: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  PENDIENTE: { label: "Pendiente", variant: "secondary" },
  EN_PROCESO: { label: "En proceso", variant: "warning" },
  COMPLETADA: { label: "Completada", variant: "success" },
  CANCELADA: { label: "Cancelada", variant: "destructive" },
};

/** Estado de la sesión de peluquería de una línea (tablero del día). */
export const GROOMING_SESSION_STATE: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  SIN_SESION: { label: "Sin ficha", variant: "secondary" },
  ABIERTA: { label: "Ficha abierta", variant: "warning" },
  EN_PROCESO: { label: "En proceso", variant: "warning" },
  FINALIZADA: { label: "Finalizada", variant: "success" },
};

/** Nivel de enredos del pelaje (0–3). */
export const GROOMING_MATTING: Record<number, string> = {
  0: "Sin enredos",
  1: "Leve",
  2: "Moderado",
  3: "Severo",
};

/** Comportamiento del paciente durante la sesión. */
export const GROOMING_BEHAVIOR_LABEL: Record<string, string> = {
  Tranquilo: "Tranquilo",
  Nervioso: "Nervioso",
  Agresivo: "Agresivo",
};

/** Tipo de baño. */
export const GROOMING_BATH_LABEL: Record<string, string> = {
  Normal: "Normal",
  Medicado: "Medicado",
  Antipulgas: "Antipulgas",
  Deshedding: "Deshedding (deslanado)",
};

/** Etiqueta del tipo de adjunto de visita. */
export const VISIT_ATTACHMENT_KIND: Record<string, string> = {
  BEFORE: "Antes",
  AFTER: "Después",
  DOCUMENT: "Documento",
};

/* ===================== ESTANCIA E INVENTARIO (fase 6) ===================== */

/** Estados de una estancia (migración 0032). */
export const STAY_STATUS: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  ABIERTA: { label: "Abierta", variant: "success" },
  CERRADA: { label: "Cerrada", variant: "secondary" },
  CANCELADA: { label: "Cancelada", variant: "destructive" },
};

/** Tipos de estancia: guardería, hospedaje u hospitalización. */
export const STAY_KIND: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  GUARDERIA: { label: "Guardería", variant: "secondary" },
  HOSPEDAJE: { label: "Hospedaje", variant: "default" },
  HOSPITALIZACION: { label: "Hospitalización", variant: "warning" },
};

/** Tipos de evento de la bitácora de cuidados. */
export const STAY_EVENT_KIND: Record<string, string> = {
  COMIDA: "Comida",
  MEDICACION: "Medicación",
  PASEO: "Paseo",
  LIMPIEZA: "Limpieza",
  NOTA: "Nota",
  VETERINARIO: "Valoración veterinaria",
};

/** Tipos de recurso (espacio físico de la clínica). */
export const RESOURCE_KIND: Record<string, string> = {
  CANIL: "Canil",
  GATO: "Gatera",
  RECEPERA: "Recupera",
  QUIRURGICO: "Quirúrgico",
  AISLAMIENTO: "Aislamiento",
  PERRERA: "Perrera",
};

/** Categorías del catálogo de productos (migración 0033). */
export const PRODUCT_CATEGORY: Record<string, string> = {
  ALIMENTO: "Alimento",
  MEDICAMENTO: "Medicamento",
  ACCESORIO: "Accesorio",
  INSUMO: "Insumo",
};

/** Tipos de movimiento de inventario. */
export const STOCK_MOVEMENT_KIND: Record<
  string,
  { label: string; variant: BadgeVariant }
> = {
  ENTRADA: { label: "Entrada", variant: "success" },
  SALIDA: { label: "Salida", variant: "destructive" },
  CONSUMO: { label: "Consumo", variant: "warning" },
  DEVOLUCION: { label: "Devolución", variant: "default" },
  AJUSTE: { label: "Ajuste", variant: "secondary" },
};

/** Unidades de medida del catálogo de productos. */
export const PRODUCT_UNIT: Record<string, string> = {
  UNIDAD: "Unidad",
  KG: "Kilogramo",
  LT: "Litro",
  PAQUETE: "Paquete",
  CAJA: "Caja",
  BOLSA: "Bolsa",
};

export const SEX_LABEL: Record<string, string> = {
  M: "Macho",
  F: "Hembra",
};

export const ROLE_LABEL: Record<string, string> = {
  ADMIN: "Administrador",
  VETERINARIO: "Veterinario",
  AUXILIAR: "Auxiliar",
  RECEPCION: "Recepción",
  PELUQUERO: "Peluquero",
};

/** Estilos del badge de rol (orden: jerárquico). */
export const ROLE_BADGE_CLASS: Record<string, string> = {
  ADMIN: "bg-purple-100 text-purple-700 dark:bg-purple-500/20 dark:text-purple-300",
  VETERINARIO: "bg-blue-100 text-blue-700 dark:bg-blue-500/20 dark:text-blue-300",
  AUXILIAR: "bg-teal-100 text-teal-700 dark:bg-teal-500/20 dark:text-teal-300",
  RECEPCION: "bg-amber-100 text-amber-700 dark:bg-amber-500/20 dark:text-amber-300",
  PELUQUERO: "bg-pink-100 text-pink-700 dark:bg-pink-500/20 dark:text-pink-300",
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
