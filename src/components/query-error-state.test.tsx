import { describe, it, expect, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { QueryErrorState } from "./query-error-state";

describe("QueryErrorState", () => {
  it("muestra qué falló, el motivo y reintenta", () => {
    const onRetry = vi.fn();
    const error = { type: "Database", data: "No se pudo conectar a Firebird" };

    render(
      <QueryErrorState
        what="la agenda de consultas"
        error={error}
        onRetry={onRetry}
      />,
    );

    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("No se pudo cargar la agenda de consultas");
    expect(alert).toHaveTextContent("No se pudo conectar a Firebird");

    fireEvent.click(screen.getByRole("button", { name: /Reintentar/i }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it("acepta un Error nativo y una pista opcional", () => {
    render(
      <QueryErrorState
        what="la configuración"
        hint="Revisa la conexión con la base de datos"
        error={new Error("boom")}
        onRetry={vi.fn()}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent("boom");
    expect(
      screen.getByText("Revisa la conexión con la base de datos"),
    ).toBeInTheDocument();
  });
});
