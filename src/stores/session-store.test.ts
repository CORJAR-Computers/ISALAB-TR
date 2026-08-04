import { describe, it, expect, beforeEach, vi } from "vitest";
import { useSessionStore } from "./session-store";

// Mock the api module
vi.mock("@/lib/api", () => ({
  api: {
    logout: vi.fn().mockResolvedValue(undefined),
  },
}));

beforeEach(() => {
  useSessionStore.setState({
    session: null,
    hydrated: false,
    changePasswordOpen: false,
  });
});

describe("useSessionStore - initial state", () => {
  it("session is null by default", () => {
    expect(useSessionStore.getState().session).toBeNull();
  });

  it("hydrated is false by default", () => {
    expect(useSessionStore.getState().hydrated).toBe(false);
  });

  it("changePasswordOpen is false by default", () => {
    expect(useSessionStore.getState().changePasswordOpen).toBe(false);
  });
});

describe("useSessionStore - setSession", () => {
  it("sets session user", () => {
    const user = {
      id: 1,
      username: "admin",
      fullName: "Admin User",
      role: "ADMIN",
      mustChangePassword: false,
    };
    useSessionStore.getState().setSession(user);
    expect(useSessionStore.getState().session).toEqual(user);
  });

  it("clears session with null", () => {
    const user = {
      id: 1,
      username: "admin",
      fullName: "Admin User",
      role: "ADMIN",
      mustChangePassword: false,
    };
    useSessionStore.getState().setSession(user);
    useSessionStore.getState().setSession(null);
    expect(useSessionStore.getState().session).toBeNull();
  });
});

describe("useSessionStore - hydration", () => {
  it("setHydrated sets hydrated flag", () => {
    useSessionStore.getState().setHydrated(true);
    expect(useSessionStore.getState().hydrated).toBe(true);
  });
});

describe("useSessionStore - change password dialog", () => {
  it("openChangePassword opens dialog", () => {
    useSessionStore.getState().openChangePassword();
    expect(useSessionStore.getState().changePasswordOpen).toBe(true);
  });

  it("closeChangePassword closes dialog", () => {
    useSessionStore.getState().openChangePassword();
    useSessionStore.getState().closeChangePassword();
    expect(useSessionStore.getState().changePasswordOpen).toBe(false);
  });
});

describe("useSessionStore - logout", () => {
  it("clears session after logout", async () => {
    const user = {
      id: 1,
      username: "admin",
      fullName: "Admin User",
      role: "ADMIN",
      mustChangePassword: false,
    };
    useSessionStore.getState().setSession(user);
    await useSessionStore.getState().logout();
    expect(useSessionStore.getState().session).toBeNull();
  });
});
