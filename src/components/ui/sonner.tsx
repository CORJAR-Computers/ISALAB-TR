import type { CSSProperties } from "react";
import { Toaster as Sonner, type ToasterProps } from "sonner";
import { useUiStore } from "@/stores/ui-store";

const Toaster = ({ ...props }: ToasterProps) => {
  const theme = useUiStore((s) => s.theme);

  return (
    <Sonner
      theme={theme === "dark" ? "dark" : "light"}
      className="toaster group"
      style={
        {
          "--normal-bg": "var(--popover)",
          "--normal-text": "var(--popover-foreground)",
          "--normal-border": "var(--border)",
        } as CSSProperties
      }
      {...props}
    />
  );
};

export { Toaster };
