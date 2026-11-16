import { useState } from "react";
import { UserPlus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { NewPatientDialog } from "./NewPatientDialog";
import type { Patient } from "@/bindings";

/**
 * Botón que abre el alta de propietario+paciente **sin abandonar el módulo
 * actual**: al guardar, entrega el paciente recién creado a un callback
 * (p. ej. para preseleccionarlo en una visita, factura u orden).
 *
 * Concepto "no te vayas": crear datos maestros en línea evita perder el
 * contexto del flujo que el usuario estaba completando.
 */
export function PatientQuickCreateButton({
  onCreated,
  label = "Nuevo paciente",
  variant = "outline",
  size = "sm",
  className,
}: {
  onCreated: (patient: Patient) => void;
  /** Cadena vacía = botón de solo icono (con aria-label). */
  label?: string;
  variant?: "default" | "outline" | "secondary" | "ghost" | "destructive";
  size?: "default" | "sm" | "lg" | "icon";
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const iconOnly = label === "";
  return (
    <>
      <Button
        type="button"
        variant={variant}
        size={size}
        className={className}
        onClick={() => setOpen(true)}
        title={iconOnly ? "Nuevo paciente" : undefined}
        aria-label={iconOnly ? "Nuevo paciente" : undefined}
      >
        <UserPlus className="size-4" />
        {!iconOnly && label}
      </Button>
      <NewPatientDialog open={open} onOpenChange={setOpen} onCreated={onCreated} />
    </>
  );
}
