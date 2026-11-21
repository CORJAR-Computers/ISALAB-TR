import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LoginPage } from "./LoginPage";

/**
 * Login: única puerta a la clínica. Se comprueba la validación local (no se
 * invoca el backend con campos vacíos), el envío limpio del usuario y que un
 * rechazo del backend no deja el formulario bloqueado.
 */

const mockLogin = vi.fn();
const mockUseClinicBranding = vi.fn();

vi.mock("@/hooks/use-queries", () => ({
  useLogin: () => mockLogin(),
  useClinicBranding: () => mockUseClinicBranding(),
}));

vi.mock("@/features/db/DbConnectionDialog", () => ({
  DbConnectionDialog: () => null,
}));

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

beforeEach(() => {
  mockLogin.mockReset();
  mockUseClinicBranding.mockReset();
  mockUseClinicBranding.mockReturnValue({ data: null });
  mockLogin.mockReturnValue({
    mutateAsync: vi.fn().mockResolvedValue({
      username: "admin",
      fullName: "Administrador",
    }),
    isPending: false,
  });
});

describe("LoginPage", () => {
  it("no llama al backend con el formulario vacío", async () => {
    render(<LoginPage />);

    await userEvent.click(screen.getByRole("button", { name: "Entrar" }));

    expect(await screen.findByText("El usuario es requerido")).toBeInTheDocument();
    expect(
      screen.getByText("La contraseña es requerida"),
    ).toBeInTheDocument();
    expect(mockLogin.mock.results[0].value.mutateAsync).not.toHaveBeenCalled();
  });

  it("envía el usuario sin espacios sobrantes", async () => {
    render(<LoginPage />);

    await userEvent.type(screen.getByLabelText("Usuario"), "  admin  ");
    await userEvent.type(screen.getByLabelText("Contraseña"), "admin123");
    await userEvent.click(screen.getByRole("button", { name: "Entrar" }));

    const mutateAsync = mockLogin.mock.results[0].value.mutateAsync;
    await waitFor(() =>
      expect(mutateAsync).toHaveBeenCalledWith({
        username: "admin",
        password: "admin123",
      }),
    );
  });

  it("muestra el motivo cuando el backend rechaza las credenciales", async () => {
    mockLogin.mockReturnValue({
      mutateAsync: vi
        .fn()
        .mockRejectedValue({ type: "Validation", data: "Usuario o contraseña incorrectos" }),
      isPending: false,
    });
    const { toast } = await import("sonner");

    render(<LoginPage />);
    await userEvent.type(screen.getByLabelText("Usuario"), "admin");
    await userEvent.type(screen.getByLabelText("Contraseña"), "mala");
    await userEvent.click(screen.getByRole("button", { name: "Entrar" }));

    await waitFor(() => expect(toast.error).toHaveBeenCalled());
    // El formulario sigue disponible para reintentar.
    expect(screen.getByRole("button", { name: "Entrar" })).toBeEnabled();
  });
});
