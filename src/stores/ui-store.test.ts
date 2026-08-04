import { describe, it, expect, beforeEach } from "vitest";
import { useUiStore, type View } from "./ui-store";

// Reset the store before each test
beforeEach(() => {
  useUiStore.setState({
    theme: "light",
    sidebarOpen: true,
    view: "dashboard",
    activePatientId: null,
    newPatientRequest: 0,
    aboutOpen: false,
  });
  // Reset DOM
  document.documentElement.classList.remove("dark");
});

describe("useUiStore - theme", () => {
  it("defaults to light theme", () => {
    expect(useUiStore.getState().theme).toBe("light");
  });

  it("setTheme changes theme", () => {
    useUiStore.getState().setTheme("dark");
    expect(useUiStore.getState().theme).toBe("dark");
  });

  it("toggleTheme switches from light to dark", () => {
    useUiStore.getState().toggleTheme();
    expect(useUiStore.getState().theme).toBe("dark");
  });

  it("toggleTheme switches from dark to light", () => {
    useUiStore.setState({ theme: "dark" });
    useUiStore.getState().toggleTheme();
    expect(useUiStore.getState().theme).toBe("light");
  });
});

describe("useUiStore - navigation", () => {
  it("defaults to dashboard view", () => {
    expect(useUiStore.getState().view).toBe("dashboard");
  });

  it("navigate changes view", () => {
    useUiStore.getState().navigate("patients" as View);
    expect(useUiStore.getState().view).toBe("patients");
  });

  it("can navigate to all views", () => {
    const views: View[] = [
      "dashboard",
      "agenda",
      "patients",
      "clinical-history",
      "samples",
      "surgeries",
      "vaccines",
      "invoices",
      "reports",
      "users",
      "audit-log",
      "settings",
    ];
    for (const view of views) {
      useUiStore.getState().navigate(view);
      expect(useUiStore.getState().view).toBe(view);
    }
  });
});

describe("useUiStore - sidebar", () => {
  it("defaults to open", () => {
    expect(useUiStore.getState().sidebarOpen).toBe(true);
  });

  it("setSidebarOpen toggles sidebar", () => {
    useUiStore.getState().setSidebarOpen(false);
    expect(useUiStore.getState().sidebarOpen).toBe(false);
    useUiStore.getState().setSidebarOpen(true);
    expect(useUiStore.getState().sidebarOpen).toBe(true);
  });
});

describe("useUiStore - active patient", () => {
  it("defaults to null", () => {
    expect(useUiStore.getState().activePatientId).toBeNull();
  });

  it("setActivePatient sets patient id", () => {
    useUiStore.getState().setActivePatient(42);
    expect(useUiStore.getState().activePatientId).toBe(42);
  });

  it("setActivePatient(null) clears patient", () => {
    useUiStore.getState().setActivePatient(42);
    useUiStore.getState().setActivePatient(null);
    expect(useUiStore.getState().activePatientId).toBeNull();
  });
});

describe("useUiStore - new patient request", () => {
  it("defaults to 0", () => {
    expect(useUiStore.getState().newPatientRequest).toBe(0);
  });

  it("requestNewPatient increments counter", () => {
    useUiStore.getState().requestNewPatient();
    expect(useUiStore.getState().newPatientRequest).toBe(1);
    useUiStore.getState().requestNewPatient();
    expect(useUiStore.getState().newPatientRequest).toBe(2);
  });

  it("consumeNewPatientRequest resets to 0", () => {
    useUiStore.getState().requestNewPatient();
    useUiStore.getState().requestNewPatient();
    useUiStore.getState().consumeNewPatientRequest();
    expect(useUiStore.getState().newPatientRequest).toBe(0);
  });
});

describe("useUiStore - about dialog", () => {
  it("defaults to closed", () => {
    expect(useUiStore.getState().aboutOpen).toBe(false);
  });

  it("setAboutOpen opens and closes", () => {
    useUiStore.getState().setAboutOpen(true);
    expect(useUiStore.getState().aboutOpen).toBe(true);
    useUiStore.getState().setAboutOpen(false);
    expect(useUiStore.getState().aboutOpen).toBe(false);
  });
});
