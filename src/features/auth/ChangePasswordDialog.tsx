import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import { KeyRound, Loader2, ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { useSessionStore } from "@/stores/session-store";
import { useChangePassword } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";

const schema = z
  .object({
    currentPassword: z.string().min(1, "Debes escribir tu contraseña actual"),
    newPassword: z
      .string()
      .min(6, "Mínimo 6 caracteres"),
    confirmPassword: z.string(),
  })
  .refine((d) => d.newPassword === d.confirmPassword, {
    message: "Las contraseñas no coinciden",
    path: ["confirmPassword"],
  });

type Values = z.infer<typeof schema>;

export function ChangePasswordDialog({
  open,
  onOpenChange,
  /** Si es `true`, el diálogo se muestra forzado (sin posibilidad de cerrar)
   *  cuando MUST_CHANGE_PASSWORD está activo. */
  forced,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  forced?: boolean;
}) {
  const changePassword = useChangePassword();
  const session = useSessionStore((s) => s.session);

  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: {
      currentPassword: "",
      newPassword: "",
      confirmPassword: "",
    },
  });

  const onSubmit = async (vals: Values) => {
    try {
      await changePassword.mutateAsync({
        currentPassword: vals.currentPassword,
        newPassword: vals.newPassword,
      });
      toast.success("Contraseña actualizada", {
        description: forced
          ? "Ya puedes usar la aplicación normalmente."
          : undefined,
      });
      form.reset();
      if (!forced) onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo cambiar la contraseña", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={forced ? () => {} : onOpenChange}
    >
      <DialogContent
        showCloseButton={!forced}
        className="sm:max-w-md"
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <KeyRound className="size-4 text-primary" />
            {forced ? "Cambio de contraseña requerido" : "Cambiar contraseña"}
          </DialogTitle>
          <DialogDescription>
            {forced
              ? `Es la primera vez que inicias sesión como ${session?.username}. Fija una contraseña segura para continuar.`
              : "Actualiza tu contraseña de acceso al sistema."}
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="currentPassword"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Contraseña actual</FormLabel>
                  <FormControl>
                    <Input
                      type="password"
                      placeholder="••••••••"
                      autoComplete="current-password"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="newPassword"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Nueva contraseña</FormLabel>
                  <FormControl>
                    <Input
                      type="password"
                      placeholder="Mín. 6 caracteres"
                      autoComplete="new-password"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="confirmPassword"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Confirmar nueva contraseña</FormLabel>
                  <FormControl>
                    <Input
                      type="password"
                      placeholder="Repite la contraseña"
                      autoComplete="new-password"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <DialogFooter>
              {!forced && (
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => onOpenChange(false)}
                >
                  Cancelar
                </Button>
              )}
              <Button
                type="submit"
                disabled={changePassword.isPending}
              >
                {changePassword.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <ShieldCheck className="size-4" />
                )}
                {forced ? "Fijar contraseña" : "Cambiar contraseña"}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}