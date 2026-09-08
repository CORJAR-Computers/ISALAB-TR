import * as React from "react";
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { XIcon } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * Radix (react-dismissable-layer) pone `body { pointer-events: none }` mientras
 * un modal está abierto y lo restaura al cerrar. Con diálogos apilados de
 * distintos `Dialog.Root` (p. ej. detalle de muestra → importar resultados),
 * la variable de módulo que guarda el valor original puede capturar "none" y,
 * al cerrar todos los modales, el body queda con `pointer-events: none`
 * atrapado: los botones del siguiente modal dejan de responder aunque el resto
 * de la app funcione. Esta red de seguridad restaura el body al cerrar (y poco
 * después, por la animación de salida) siempre que no quede ningún diálogo abierto.
 */
function fixBodyPointerEvents() {
  if (document.body.style.pointerEvents !== "none") return;
  const anyOpenDialog = document.querySelector(
    '[data-slot="dialog-content"][data-state="open"]',
  );
  if (!anyOpenDialog) document.body.style.pointerEvents = "";
}

function useRestoreBodyPointerEvents(isOpen: boolean) {
  React.useEffect(() => {
    if (!isOpen) fixBodyPointerEvents();
  }, [isOpen]);
  React.useEffect(() => {
    // Sin cleanup a propósito: el temporizador debe sobrevivir al desmontaje
    // para cubrir el caso de un diálogo que se desmonta estando abierto.
    window.setTimeout(fixBodyPointerEvents, 350);
  }, []);
}

function Dialog(props: React.ComponentProps<typeof DialogPrimitive.Root>) {
  useRestoreBodyPointerEvents(props.open ?? false);
  return <DialogPrimitive.Root data-slot="dialog" {...props} />;
}

function DialogTrigger(
  props: React.ComponentProps<typeof DialogPrimitive.Trigger>,
) {
  return <DialogPrimitive.Trigger data-slot="dialog-trigger" {...props} />;
}

function DialogPortal(
  props: React.ComponentProps<typeof DialogPrimitive.Portal>,
) {
  return <DialogPrimitive.Portal data-slot="dialog-portal" {...props} />;
}

function DialogClose(
  props: React.ComponentProps<typeof DialogPrimitive.Close>,
) {
  return <DialogPrimitive.Close data-slot="dialog-close" {...props} />;
}

function DialogOverlay({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Overlay>) {
  return (
    <DialogPrimitive.Overlay
      data-slot="dialog-overlay"
      className={cn(
        "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/50 backdrop-blur-sm",
        className,
      )}
      {...props}
    />
  );
}

function DialogContent({
  className,
  children,
  showCloseButton = true,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Content> & {
  showCloseButton?: boolean;
}) {
  return (
    <DialogPortal data-slot="dialog-portal">
      <DialogOverlay />
      <DialogPrimitive.Content
        data-slot="dialog-content"
        className={cn(
          "bg-background data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed top-[50%] left-[50%] z-50 grid w-full max-w-[calc(100%-2rem)] translate-x-[-50%] translate-y-[-50%] gap-4 rounded-xl border p-6 shadow-xl duration-200 sm:max-w-lg",
          className,
        )}
        // Evita que un clic fuera del modal lo cierre: en producción, al hacer
        // clic en el overlay Radix dejaba `pointer-events: none` atrapado en el
        // body, inhabilitando los botones del modal (Guardar/Cancelar) hasta
        // reabrir el diálogo. El usuario cierra explícitamente con la X, Esc o
        // los botones de acción.
        onPointerDownOutside={(e) => e.preventDefault()}
        onInteractOutside={(e) => e.preventDefault()}
        {...props}
      >
        {children}
        {showCloseButton && (
          <DialogPrimitive.Close
            data-slot="dialog-close"
            className="ring-offset-background focus:ring-ring data-[state=open]:bg-accent data-[state=open]:text-muted-foreground absolute top-4 right-4 rounded-xs opacity-70 transition-opacity hover:opacity-100 focus:ring-2 focus:ring-offset-2 focus:outline-hidden disabled:pointer-events-none [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4"
          >
            <XIcon />
            <span className="sr-only">Cerrar</span>
          </DialogPrimitive.Close>
        )}
      </DialogPrimitive.Content>
    </DialogPortal>
  );
}

function DialogHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="dialog-header"
      className={cn("flex flex-col gap-2 text-center sm:text-left", className)}
      {...props}
    />
  );
}

function DialogFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="dialog-footer"
      className={cn(
        "flex flex-col-reverse gap-2 sm:flex-row sm:justify-end",
        className,
      )}
      {...props}
    />
  );
}

function DialogTitle({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Title>) {
  return (
    <DialogPrimitive.Title
      data-slot="dialog-title"
      className={cn("text-lg leading-none font-semibold", className)}
      {...props}
    />
  );
}

function DialogDescription({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Description>) {
  return (
    <DialogPrimitive.Description
      data-slot="dialog-description"
      className={cn("text-muted-foreground text-sm", className)}
      {...props}
    />
  );
}

export {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogOverlay,
  DialogPortal,
  DialogTitle,
  DialogTrigger,
};

// Exportados para las pruebas unitarias de la red de seguridad.
export { fixBodyPointerEvents, useRestoreBodyPointerEvents };
