import { describe, it, expect } from "vitest";
import { getErrorMessage } from "./api";

describe("getErrorMessage", () => {
  it("extracts message from Error objects", () => {
    expect(getErrorMessage(new Error("Something broke"))).toBe(
      "Something broke"
    );
  });

  it("extracts message from AppError with type and data", () => {
    const err = { type: "Validation", data: "Name is required" };
    expect(getErrorMessage(err)).toBe("Validation: Name is required");
  });

  it("extracts message from object with message property", () => {
    expect(getErrorMessage({ message: "Custom error" })).toBe("Custom error");
  });

  it("returns String(e) for string errors", () => {
    expect(getErrorMessage("simple string error")).toBe("simple string error");
  });

  it("returns fallback for null", () => {
    expect(getErrorMessage(null)).toBe("Error desconocido");
  });

  it("returns fallback for undefined", () => {
    expect(getErrorMessage(undefined)).toBe("Error desconocido");
  });

  it("returns String representation for number errors", () => {
    expect(getErrorMessage(42)).toBe("42");
  });

  it("handles AppError with Db type", () => {
    const err = { type: "Db", data: "Connection refused" };
    expect(getErrorMessage(err)).toBe("Db: Connection refused");
  });

  it("handles AppError with NotFound type", () => {
    const err = { type: "NotFound", data: "Patient #42" };
    expect(getErrorMessage(err)).toBe("NotFound: Patient #42");
  });

  it("handles AppError with Forbidden type", () => {
    const err = { type: "Forbidden", data: "Admin only" };
    expect(getErrorMessage(err)).toBe("Forbidden: Admin only");
  });

  it("handles AppError with Internal type", () => {
    const err = { type: "Internal", data: "Panic in module" };
    expect(getErrorMessage(err)).toBe("Internal: Panic in module");
  });

  it("returns String(e) for empty message", () => {
    expect(getErrorMessage({ message: "" })).toBe("[object Object]");
  });

  it("handles objects without message or type", () => {
    expect(getErrorMessage({ foo: "bar" })).toBe("[object Object]");
  });
});
