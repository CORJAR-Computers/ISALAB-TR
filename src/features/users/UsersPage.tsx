import { useState } from "react";
import { toast } from "sonner";
import {
  KeyRound,
  Loader2,
  Plus,
  Shield,
  ShieldAlert,
  UserCheck,
  UserPlus,
  Users,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useCreateUser, useUsers } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import { usePermissions } from "@/hooks/use-permissions";
import { ROLE_LABEL } from "@/lib/status";
import { useSessionStore } from "@/stores/session-store";
import type { CreateUserInput } from "@/bindings";

const createSchema = z.object({
  username: z
    .string()
    .min(3, "Mínimo 3 caracteres")
    .max(40, "Máximo 40 caracteres"),
  fullName: z.string().min(3, "Nombre completo requerido"),
  role: z.enum(["ADMIN", "VETERINARIO", "AUXILIAR"]),
  initialPassword: z
    .string()
    .min(6, "Mínimo 6 caracteres"),
});

type CreateValues = z.infer<typeof createSchema>;

function CreateUserDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
}) {
  const createUser = useCreateUser();

  const form = useForm<CreateValues>({
    resolver: zodResolver(createSchema),
    defaultValues: {
      username: "",
      fullName: "",
      role: "VETERINARIO",
      initialPassword: "",
    },
  });

  const onSubmit = async (vals: CreateValues) => {
    const input: CreateUserInput = {
      username: vals.username.trim(),
      fullName: vals.fullName.trim(),
      role: vals.role,
      initialPassword: vals.initialPassword,
    };
    try {
      const user = await createUser.mutateAsync(input);
      toast.success(`Usuario ${user.fullName} creado`, {
        description: `@${user.username} · ${ROLE_LABEL[user.role] ?? user.role}`,
      });
      form.reset();
      onOpenChange(false);
    } catch (e) {
      toast.error("No se pudo crear el usuario", {
        description: getErrorMessage(e),
      });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Nuevo usuario</DialogTitle>
          <DialogDescription>
            Crea un usuario con rol y contraseña inicial. Al iniciar sesión
            deberá cambiarla.
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            <FormField
              control={form.control}
              name="username"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Usuario</FormLabel>
                  <FormControl>
                    <Input placeholder="ej. jperez" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="fullName"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Nombre completo</FormLabel>
                  <FormControl>
                    <Input placeholder="Nombre y apellidos" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="role"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Rol</FormLabel>
                  <Select value={field.value} onValueChange={field.onChange}>
                    <FormControl>
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      <SelectItem value="ADMIN">
                        <span className="flex items-center gap-2">
                          <ShieldAlert className="size-3.5 text-destructive" />
                          Administrador
                        </span>
                      </SelectItem>
                      <SelectItem value="VETERINARIO">
                        <span className="flex items-center gap-2">
                          <Shield className="size-3.5 text-primary" />
                          Veterinario
                        </span>
                      </SelectItem>
                      <SelectItem value="AUXILIAR">
                        <span className="flex items-center gap-2">
                          <UserCheck className="size-3.5 text-muted-foreground" />
                          Auxiliar
                        </span>
                      </SelectItem>
                    </SelectContent>
                  </Select>
                  <FormMessage />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="initialPassword"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Contraseña inicial</FormLabel>
                  <FormControl>
                    <Input
                      type="password"
                      placeholder="Mín. 6 caracteres"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
              >
                Cancelar
              </Button>
              <Button type="submit" disabled={createUser.isPending}>
                {createUser.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : (
                  <UserPlus className="size-4" />
                )}
                Crear usuario
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}

export function UsersPage() {
  const [createOpen, setCreateOpen] = useState(false);
  const { data: users, isLoading } = useUsers();
  const session = useSessionStore((s) => s.session);
  const openChangePassword = useSessionStore((s) => s.openChangePassword);
  const { isAdmin } = usePermissions();

  const roleVariants: Record<string, "default" | "secondary" | "outline"> = {
    ADMIN: "default",
    VETERINARIO: "secondary",
    AUXILIAR: "outline",
  };

  if (!isAdmin) {
    return (
      <div className="flex flex-col items-center justify-center gap-3 py-20">
        <ShieldAlert className="size-10 text-muted-foreground" />
        <p className="text-muted-foreground text-sm">
          Solo los administradores pueden gestionar usuarios.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold tracking-tight">Usuarios</h2>
          <p className="text-muted-foreground text-sm">
            Gestión de cuentas del sistema y roles.
          </p>
        </div>
        <Button onClick={() => setCreateOpen(true)}>
          <Plus className="size-4" />
          Nuevo usuario
        </Button>
      </div>

      <CreateUserDialog open={createOpen} onOpenChange={setCreateOpen} />

      {/* cambio de contraseña */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <KeyRound className="size-4 text-primary" />
            Tu cuenta
          </CardTitle>
          <CardDescription>
            Sesión activa como{" "}
            <span className="font-medium">{session?.fullName}</span> (
            @{session?.username})
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button variant="outline" onClick={openChangePassword}>
            <KeyRound className="size-4" />
            Cambiar contraseña
          </Button>
        </CardContent>
      </Card>

      <Separator />

      {/* listado de usuarios */}
      <div>
        <h3 className="flex items-center gap-2 text-sm font-semibold">
          <Users className="size-4 text-muted-foreground" />
          Usuarios del sistema
        </h3>
        <p className="text-muted-foreground mt-0.5 text-xs">
          Solo visible para administradores.
        </p>
      </div>

      {isLoading && !users ? (
        <Skeleton className="h-40 w-full" />
      ) : (
        <div className="rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Usuario</TableHead>
                <TableHead>Nombre</TableHead>
                <TableHead>Rol</TableHead>
                <TableHead>Estado</TableHead>
                <TableHead>Contraseña</TableHead>
                <TableHead>Creado</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {users?.length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={6}
                    className="text-muted-foreground text-center"
                  >
                    No hay usuarios registrados.
                  </TableCell>
                </TableRow>
              ) : (
                users?.map((u) => (
                  <TableRow key={u.id}>
                    <TableCell className="font-mono text-xs">
                      @{u.username}
                    </TableCell>
                    <TableCell>{u.fullName}</TableCell>
                    <TableCell>
                      <Badge variant={roleVariants[u.role] ?? "outline"}>
                        {ROLE_LABEL[u.role] ?? u.role}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      {u.active ? (
                        <Badge variant="success">Activo</Badge>
                      ) : (
                        <Badge variant="destructive">Inactivo</Badge>
                      )}
                    </TableCell>
                    <TableCell>
                      {u.mustChangePassword ? (
                        <Badge variant="warning">Pendiente</Badge>
                      ) : (
                        <Badge variant="secondary">Establecida</Badge>
                      )}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {u.createdAt}
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  );
}