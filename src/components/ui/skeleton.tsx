import { cn } from "@/lib/utils";

function Skeleton({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="skeleton"
      className={cn(
        "animate-shimmer relative overflow-hidden rounded-md bg-accent",
        className,
      )}
      style={{
        backgroundImage:
          "linear-gradient(90deg, transparent, oklch(1 0 0 / 0.35), transparent)",
        backgroundSize: "200% 100%",
      }}
      {...props}
    />
  );
}

export { Skeleton };
