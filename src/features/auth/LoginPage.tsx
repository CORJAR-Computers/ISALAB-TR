import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { toast } from "sonner";
import {
  Eye,
  EyeOff,
  FlaskConical,
  HeartPulse,
  KeyRound,
  Loader2,
  ShieldCheck,
  Stethoscope,
  User,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { useLogin } from "@/hooks/use-queries";
import { getErrorMessage } from "@/lib/api";
import logoSidebar from "@/assets/logo_sidebar.png";

const loginSchema = z.object({
  username: z.string().min(1, "El usuario es requerido"),
  password: z.string().min(1, "La contraseña es requerida"),
});

type LoginValues = z.infer<typeof loginSchema>;

const FEATURES = [
  { icon: Stethoscope, text: "Historiales clínicos por paciente" },
  { icon: FlaskConical, text: "Muestras de laboratorio con trazabilidad" },
  { icon: HeartPulse, text: "Reportes PDF generados localmente" },
];

export function LoginPage() {
  const login = useLogin();
  const [showPassword, setShowPassword] = useState(false);

  const form = useForm<LoginValues>({
    resolver: zodResolver(loginSchema),
    defaultValues: {
      username: "",
      password: "",
    },
  });

  const onSubmit = async (values: LoginValues) => {
    try {
      const user = await login.mutateAsync({
        username: values.username.trim(),
        password: values.password,
      });
      toast.success(`Bienvenido, ${user.fullName || user.username}`);
    } catch (err) {
      toast.error("No se pudo iniciar sesión", {
        description: getErrorMessage(err),
      });
    }
  };

  return (
    <div className="relative flex min-h-svh overflow-x-hidden">
      {/* Fondo decorativo */}
      <div className="bg-primary/20 absolute -top-32 -left-32 size-96 rounded-full blur-3xl animate-glow-pulse" />
      <div className="bg-success/15 absolute -right-32 -bottom-32 size-96 rounded-full blur-3xl animate-glow-pulse [animation-delay:1.5s]" />

      {/* Panel de marca */}
      <div className="from-primary via-primary/90 to-emerald-700 relative hidden w-1/2 flex-col justify-between overflow-hidden bg-gradient-to-br p-10 text-primary-foreground lg:flex">
        <div className="absolute inset-0 opacity-20">
          <div className="bg-white/10 absolute top-1/4 left-1/3 size-64 rounded-full blur-2xl animate-float" />
          <div className="bg-white/10 absolute right-1/4 bottom-1/4 size-48 rounded-full blur-2xl animate-float [animation-delay:2s]" />
        </div>

        <div className="relative">
          <div className="flex items-center gap-3">
            <div className="bg-white/15 flex size-12 items-center justify-center rounded-2xl shadow-lg backdrop-blur-sm ring-1 ring-white/20">
              <img
                src={logoSidebar}
                alt="ISALAB"
                className="max-h-full w-full object-contain"
                draggable={false}
              />
            </div>
            <div>
              <p className="text-xl font-bold tracking-wide">ISALAB</p>
              <p className="text-white/70 text-xs">
                Laboratorio Veterinario
              </p>
            </div>
          </div>
        </div>

        <div className="relative space-y-8">
          <div>
            <h2 className="text-4xl leading-tight font-bold">
              Gestión clínica
              <br />
              <span className="text-white/90">en un solo lugar</span>
            </h2>
            <p className="mt-4 max-w-md text-sm leading-relaxed text-white/75">
              Pacientes, consultas, laboratorio, vacunación, cirugías y
              facturación con datos 100 % locales: seguros y siempre
              disponibles.
            </p>
          </div>

          <ul className="space-y-3">
            {FEATURES.map((f) => (
              <li
                key={f.text}
                className="flex items-center gap-3 text-sm text-white/85"
              >
                <span className="bg-white/15 flex size-8 items-center justify-center rounded-lg backdrop-blur-sm ring-1 ring-white/15">
                  <f.icon className="size-4" />
                </span>
                {f.text}
              </li>
            ))}
          </ul>
        </div>

        <div className="text-white/50 relative flex items-center gap-2 text-xs">
          <ShieldCheck className="size-3.5" />
          Firebird 5 Embedded · Tauri v2 · datos locales
        </div>
      </div>

      {/* Panel de acceso */}
      <div className="flex w-full items-center justify-center bg-background px-4 py-10 lg:w-1/2">
        <div className="animate-fade-in-up w-full max-w-sm">
          <div className="mb-8 flex flex-col items-center gap-3 text-center lg:hidden">
            <div className="bg-card flex size-16 items-center justify-center rounded-2xl border p-2 shadow-md">
              <img
                src={logoSidebar}
                alt="ISALAB · Laboratorio Veterinario"
                className="h-auto max-h-full w-full object-contain"
                draggable={false}
              />
            </div>
            <div>
              <h1 className="text-xl font-bold tracking-wide">ISALAB</h1>
              <p className="text-muted-foreground text-sm">
                Laboratorio Veterinario · Inicia sesión
              </p>
            </div>
          </div>

          <div className="lg:hidden">
            <h1 className="text-2xl font-bold tracking-tight">
              Bienvenido de nuevo
            </h1>
            <p className="text-muted-foreground mt-1 text-sm">
              Ingresa tus credenciales para continuar.
            </p>
          </div>

          <Form {...form}>
            <form
              onSubmit={form.handleSubmit(onSubmit)}
              className="bg-card border-card mt-6 rounded-2xl border p-6 shadow-lg transition-shadow hover:shadow-xl"
            >
              <div className="space-y-4">
                <FormField
                  control={form.control}
                  name="username"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Usuario</FormLabel>
                      <div className="relative">
                        <User className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                        <FormControl>
                          <Input
                            placeholder="admin"
                            autoComplete="username"
                            className="pl-9"
                            autoFocus
                            {...field}
                          />
                        </FormControl>
                      </div>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                  control={form.control}
                  name="password"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Contraseña</FormLabel>
                      <div className="relative">
                        <KeyRound className="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" />
                        <FormControl>
                          <Input
                            type={showPassword ? "text" : "password"}
                            placeholder="••••••••"
                            autoComplete="current-password"
                            className="pl-9 pr-9"
                            {...field}
                          />
                        </FormControl>
                        <button
                          type="button"
                          onClick={() => setShowPassword((v) => !v)}
                          className="text-muted-foreground hover:text-foreground absolute top-1/2 right-2.5 -translate-y-1/2"
                          aria-label={
                            showPassword ? "Ocultar contraseña" : "Mostrar contraseña"
                          }
                        >
                          {showPassword ? (
                            <EyeOff className="size-4" />
                          ) : (
                            <Eye className="size-4" />
                          )}
                        </button>
                      </div>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <Button
                  type="submit"
                  className="w-full"
                  disabled={login.isPending}
                >
                  {login.isPending ? (
                    <Loader2 className="animate-spin" />
                  ) : (
                    <KeyRound className="size-4" />
                  )}
                  Entrar
                </Button>
              </div>

              <p className="text-muted-foreground mt-5 text-center text-xs">
                Primer acceso: <code className="font-mono">admin</code> /{" "}
                <code className="font-mono">admin123</code>
              </p>
            </form>
          </Form>

          <p className="text-muted-foreground mt-6 flex items-center justify-center gap-1.5 text-center text-xs">
            <ShieldCheck className="size-3.5" />
            Tus datos nunca salen de este equipo
          </p>
        </div>
      </div>
    </div>
  );
}
