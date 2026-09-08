import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen, act, cleanup } from "@testing-library/react";
import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogTrigger,
  fixBodyPointerEvents,
  useRestoreBodyPointerEvents,
} from "./dialog";

// La red de seguridad restaura `body { pointer-events }` cuando el último
// diálogo se cierra. Sin ella, Radix puede dejar "none" atrapado y la app
// entera queda "fantasma" (los botones no responden aunque se vean).
describe("fixBodyPointerEvents", () => {
  afterEach(() => {
    document.body.style.pointerEvents = "";
    vi.useRealTimers();
    cleanup();
  });

  it("restaura el body cuando hay pointer-events: none y no hay diálogos abiertos", () => {
    document.body.style.pointerEvents = "none";
    fixBodyPointerEvents();
    expect(document.body.style.pointerEvents).toBe("");
  });

  it("no toca el body si pointer-events no es none", () => {
    document.body.style.pointerEvents = "auto";
    fixBodyPointerEvents();
    expect(document.body.style.pointerEvents).toBe("auto");
  });

  it("no toca el body si el estilo está vacío", () => {
    document.body.style.pointerEvents = "";
    fixBodyPointerEvents();
    expect(document.body.style.pointerEvents).toBe("");
  });

  it("conserva pointer-events: none mientras quede un diálogo abierto", () => {
    document.body.style.pointerEvents = "none";
    document.body.innerHTML =
      '<div data-slot="dialog-content" data-state="open"></div>';
    fixBodyPointerEvents();
    expect(document.body.style.pointerEvents).toBe("none");
  });

  it("restaura el body cuando el único diálogo abierto ya se cerró", () => {
    document.body.style.pointerEvents = "none";
    document.body.innerHTML =
      '<div data-slot="dialog-content" data-state="closed"></div>';
    fixBodyPointerEvents();
    expect(document.body.style.pointerEvents).toBe("");
  });
});

// El hook dispara la restauración al pasar de abierto→cerrado y poco después
// (temporizador superviviente) para cubrir diálogos que se desmontan abiertos.
describe("useRestoreBodyPointerEvents", () => {
  afterEach(() => {
    document.body.style.pointerEvents = "";
    vi.useRealTimers();
    cleanup();
  });

  it("restaura al pasar de abierto a cerrado", () => {
    function Harness({ open }: { open: boolean }) {
      useRestoreBodyPointerEvents(open);
      return null;
    }
    document.body.style.pointerEvents = "none";
    const { rerender } = render(<Harness open />);
    rerender(<Harness open={false} />);
    expect(document.body.style.pointerEvents).toBe("");
  });

  it("no restaura mientras el diálogo sigue abierto", () => {
    function Harness({ open }: { open: boolean }) {
      useRestoreBodyPointerEvents(open);
      return null;
    }
    document.body.style.pointerEvents = "none";
    const { rerender } = render(<Harness open />);
    rerender(<Harness open />);
    expect(document.body.style.pointerEvents).toBe("none");
  });

  it("el temporizador superviviente restaura tras desmontar el diálogo abierto", () => {
    vi.useFakeTimers();
    function Harness({ open }: { open: boolean }) {
      useRestoreBodyPointerEvents(open);
      return null;
    }
    document.body.style.pointerEvents = "none";
    const { unmount } = render(<Harness open />);
    // Se desmonta estando abierto (caso del bug: stack de modales desmontado
    // de golpe). El temporizador debe sobrevivir al unmount.
    unmount();
    act(() => {
      vi.advanceTimersByTime(400);
    });
    expect(document.body.style.pointerEvents).toBe("");
  });

  it("monta un único temporizador por instancia", () => {
    vi.useFakeTimers();
    const setTimeoutSpy = vi.spyOn(window, "setTimeout");
    function Harness() {
      useRestoreBodyPointerEvents(true);
      return null;
    }
    render(<Harness />);
    const timerCalls = setTimeoutSpy.mock.calls.length;
    expect(timerCalls).toBe(1);
    setTimeoutSpy.mockRestore();
  });
});

// Integración con el componente Dialog: el ciclo abrir→cerrar del modal real
// deja el body interactivo (regresión del bug de modales fantasma).
describe("Dialog (integración de pointer-events)", () => {
  afterEach(() => {
    document.body.style.pointerEvents = "";
    vi.useRealTimers();
    cleanup();
  });

  it("al cerrar el diálogo el body vuelve a ser interactivo", () => {
    function Harness() {
      const [open, setOpen] = useState(true);
      return (
        <Dialog open={open} onOpenChange={setOpen}>
          <DialogTrigger>abrir</DialogTrigger>
          <DialogContent>
            <button onClick={() => setOpen(false)}>cerrar</button>
          </DialogContent>
        </Dialog>
      );
    }
    document.body.style.pointerEvents = "none";
    render(<Harness />);

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    act(() => {
      screen.getByRole("button", { name: "cerrar" }).click();
    });
    // El estado del contenido pasa a closed con el diálogo aún montado
    // (animación de salida); la red de seguridad ya puede restaurar porque
    // ningún contenido sigue en estado open.
    expect(
      document.querySelector('[data-slot="dialog-content"][data-state="open"]'),
    ).toBeNull();
    expect(document.body.style.pointerEvents).toBe("");
  });
});
