import type { LucideIcon } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";

export function PlaceholderPage({
  icon: Icon,
  title,
  description,
}: {
  icon: LucideIcon;
  title: string;
  description: string;
}) {
  return (
    <Card>
      <CardContent className="flex flex-col items-center gap-4 py-16 text-center">
        <div className="bg-muted flex size-14 items-center justify-center rounded-2xl">
          <Icon className="text-muted-foreground size-7" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">{title}</h2>
          <p className="text-muted-foreground mx-auto mt-1 max-w-md text-sm">
            {description}
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
