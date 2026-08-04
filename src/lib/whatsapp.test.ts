import { describe, it, expect, vi } from "vitest";
import { formatWhatsAppNumber, sendWhatsAppMessage } from "./whatsapp";

describe("formatWhatsAppNumber", () => {
  it("returns empty string for null/undefined", () => {
    expect(formatWhatsAppNumber(null)).toBe("");
    expect(formatWhatsAppNumber(undefined)).toBe("");
  });

  it("returns empty string for empty string", () => {
    expect(formatWhatsAppNumber("")).toBe("");
  });

  it("adds 57 prefix to 10-digit Colombian mobile starting with 3", () => {
    expect(formatWhatsAppNumber("3001234567")).toBe("573001234567");
  });

  it("does not add prefix if number already has country code", () => {
    expect(formatWhatsAppNumber("573001234567")).toBe("573001234567");
  });

  it("does not add prefix to 10-digit number not starting with 3", () => {
    expect(formatWhatsAppNumber("1001234567")).toBe("1001234567");
  });

  it("strips non-numeric characters", () => {
    expect(formatWhatsAppNumber("+57 300 123 4567")).toBe("573001234567");
  });

  it("strips dashes and spaces", () => {
    expect(formatWhatsAppNumber("300-123-4567")).toBe("573001234567");
  });

  it("handles number with parentheses", () => {
    expect(formatWhatsAppNumber("(300) 123-4567")).toBe("573001234567");
  });

  it("handles international format with +", () => {
    expect(formatWhatsAppNumber("+573001234567")).toBe("573001234567");
  });
});

describe("sendWhatsAppMessage", () => {
  it("constructs correct WhatsApp URL", () => {
    const openSpy = vi.spyOn(window, "open").mockImplementation(() => null as any);
    try {
      sendWhatsAppMessage("3001234567", "Hola Mundo");
      expect(openSpy).toHaveBeenCalledTimes(1);
      const url = openSpy.mock.calls[0]?.[0] as string;
      expect(url).toContain("https://wa.me/573001234567");
      expect(url).toContain("text=Hola%20Mundo");
    } finally {
      openSpy.mockRestore();
    }
  });

  it("opens in a new tab (_blank)", () => {
    const openSpy = vi.spyOn(window, "open").mockImplementation(() => null as any);
    try {
      sendWhatsAppMessage("3001234567", "Test");
      expect(openSpy.mock.calls[0]?.[1]).toBe("_blank");
    } finally {
      openSpy.mockRestore();
    }
  });

  it("does not open if phone is empty", () => {
    const openSpy = vi.spyOn(window, "open").mockImplementation(() => null as any);
    try {
      sendWhatsAppMessage("", "Test");
      expect(openSpy).not.toHaveBeenCalled();
    } finally {
      openSpy.mockRestore();
    }
  });

  it("encodes special characters in message", () => {
    const openSpy = vi.spyOn(window, "open").mockImplementation(() => null as any);
    try {
      sendWhatsAppMessage("3001234567", "Hola & Adiós");
      const url = openSpy.mock.calls[0]?.[0] as string;
      expect(url).toContain("Hola%20%26%20Adi%C3%B3s");
    } finally {
      openSpy.mockRestore();
    }
  });
});
