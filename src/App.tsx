import { useQuery } from "@tanstack/react-query";
import { commands, type HealthStatus } from "@/lib/tauri";
import { Button } from "@/components/ui/button";

export function App() {
  const health = useQuery<HealthStatus>({
    queryKey: ["health"],
    queryFn: () => commands.healthCheck(),
  });

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-6 bg-background text-foreground">
      <div className="flex flex-col items-center gap-2">
        <h1 className="text-3xl font-semibold tracking-tight">Hugo Studio</h1>
        <p className="text-sm text-muted-foreground">
          Desktop editor for Hugo sites — M0 bootstrap
        </p>
      </div>

      <div className="flex flex-col items-center gap-2 rounded-lg border bg-card px-6 py-4 shadow-sm">
        {health.isPending && (
          <span className="text-sm text-muted-foreground">
            Checking backend…
          </span>
        )}
        {health.isError && (
          <span className="text-sm text-destructive">
            Backend unreachable: {String(health.error)}
          </span>
        )}
        {health.data && (
          <>
            <span className="text-2xl font-medium">
              {health.data.status === "ready" ? "ready" : health.data.status}
            </span>
            <span className="text-xs text-muted-foreground">
              v{health.data.version}
            </span>
          </>
        )}
      </div>

      <Button variant="outline" size="sm" onClick={() => health.refetch()}>
        Re-check
      </Button>
    </main>
  );
}
