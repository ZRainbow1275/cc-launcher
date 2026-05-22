import { Suspense, lazy, type ReactNode } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { ThemeProvider } from "@/components/theme-provider";
import { UpdateProvider } from "@/contexts/UpdateContext";
import { Toaster } from "@/components/ui/sonner";
import { queryClient } from "@/lib/query";
import "@/i18n";

const App = lazy(() => import("@/App"));

function LoadingSkeleton() {
  return (
    <div
      data-testid="root-shell-loading"
      className="flex min-h-screen w-full items-center justify-center bg-background"
    >
      <div className="flex flex-col items-center gap-3">
        <div className="h-10 w-10 rounded-full border-2 border-muted border-t-foreground animate-spin" />
        <div className="text-sm text-muted-foreground">Loading...</div>
      </div>
    </div>
  );
}

export interface RootShellProps {
  children?: ReactNode;
}

export function RootShell(_props: RootShellProps = {}) {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider defaultTheme="system" storageKey="cc-switch-theme">
        <UpdateProvider>
          <Suspense fallback={<LoadingSkeleton />}>
            <App />
          </Suspense>
          <Toaster />
        </UpdateProvider>
      </ThemeProvider>
    </QueryClientProvider>
  );
}

export default RootShell;
