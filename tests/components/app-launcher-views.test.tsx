import { screen, waitFor, within, act } from "@testing-library/react";
import userEvent, {
  PointerEventsCheckLevel,
} from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Stub matchMedia for jsdom (ThemeProvider relies on it transitively).
if (typeof window !== "undefined" && !window.matchMedia) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}

// ---------------------------------------------------------------------------
// Heavy-child mocks. App.tsx renders ~14 panels; we replace them with thin
// stand-ins so the test can focus on the new launcher-view routing behavior
// (View type, header button, back button, sequence, onboarding redirect)
// without paying the cost of provider lists, dialogs, sessions, etc.
// ---------------------------------------------------------------------------

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
  },
  Toaster: () => null,
}));

vi.mock("framer-motion", () => {
  const React = require("react") as typeof import("react");
  const motion = new Proxy(
    {},
    {
      get: () => {
        const Comp = React.forwardRef<HTMLDivElement, Record<string, unknown>>(
          ({ children, ...rest }, ref) => (
            <div ref={ref} {...(rest as Record<string, unknown>)}>
              {children as React.ReactNode}
            </div>
          ),
        );
        Comp.displayName = "MotionStub";
        return Comp;
      },
    },
  );
  return {
    motion,
    AnimatePresence: ({ children }: { children: React.ReactNode }) => (
      <>{children}</>
    ),
  };
});

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    setDecorations: async () => {},
    isMaximized: async () => false,
    onResized: async () => () => {},
    minimize: async () => {},
    toggleMaximize: async () => {},
    close: async () => {},
  }),
}));

vi.mock("@/components/providers/ProviderList", () => ({
  ProviderList: () => <div data-testid="mock-provider-list">providers</div>,
}));

vi.mock("@/components/providers/AddProviderDialog", () => ({
  AddProviderDialog: () => null,
}));

vi.mock("@/components/providers/EditProviderDialog", () => ({
  EditProviderDialog: () => null,
}));

vi.mock("@/components/AppSwitcher", () => ({
  AppSwitcher: () => <div data-testid="mock-app-switcher" />,
}));

vi.mock("@/components/settings/SettingsPage", () => ({
  SettingsPage: () => <div data-testid="mock-settings-page">settings</div>,
}));

vi.mock("@/components/UpdateBadge", () => ({
  UpdateBadge: () => <div data-testid="mock-update-badge" />,
}));

vi.mock("@/components/env/EnvWarningBanner", () => ({
  EnvWarningBanner: () => null,
}));

vi.mock("@/components/proxy/ProxyToggle", () => ({
  ProxyToggle: () => null,
}));

vi.mock("@/components/proxy/ClaudeDesktopRouteToggle", () => ({
  ClaudeDesktopRouteToggle: () => null,
}));

vi.mock("@/components/proxy/FailoverToggle", () => ({
  FailoverToggle: () => null,
}));

vi.mock("@/components/UsageScriptModal", () => ({
  default: () => null,
}));

vi.mock("@/components/mcp/UnifiedMcpPanel", () => ({
  __esModule: true,
  default: () => null,
}));

vi.mock("@/components/prompts/PromptPanel", () => ({
  __esModule: true,
  default: () => null,
}));

vi.mock("@/components/skills/SkillsPage", () => ({
  SkillsPage: () => null,
}));

vi.mock("@/components/skills/UnifiedSkillsPanel", () => ({
  __esModule: true,
  default: () => null,
}));

vi.mock("@/components/DeepLinkImportDialog", () => ({
  DeepLinkImportDialog: () => null,
}));

vi.mock("@/components/FirstRunNoticeDialog", () => ({
  FirstRunNoticeDialog: () => null,
}));

vi.mock("@/components/agents/AgentsPanel", () => ({
  AgentsPanel: () => null,
}));

vi.mock("@/components/universal", () => ({
  UniversalProviderPanel: () => null,
}));

vi.mock("@/components/BrandIcons", () => ({
  McpIcon: () => null,
}));

vi.mock("@/components/sessions/SessionManagerPage", () => ({
  SessionManagerPage: () => null,
}));

vi.mock("@/components/workspace/WorkspaceFilesPanel", () => ({
  __esModule: true,
  default: () => null,
}));

vi.mock("@/components/openclaw/EnvPanel", () => ({
  __esModule: true,
  default: () => null,
}));

vi.mock("@/components/openclaw/ToolsPanel", () => ({
  __esModule: true,
  default: () => null,
}));

vi.mock("@/components/openclaw/AgentsDefaultsPanel", () => ({
  __esModule: true,
  default: () => null,
}));

vi.mock("@/components/openclaw/OpenClawHealthBanner", () => ({
  __esModule: true,
  default: () => null,
}));

vi.mock("@/components/hermes/HermesMemoryPanel", () => ({
  __esModule: true,
  default: () => null,
}));

// Launcher panels: lightweight stand-ins so we can assert routing.
vi.mock("@/components/installer", () => ({
  InstallerWizard: () => (
    <div data-testid="mock-installer-wizard">installer</div>
  ),
}));

vi.mock("@/components/profile/ProfileManager", () => ({
  ProfileManager: () => (
    <div data-testid="mock-profile-manager">profile-manager</div>
  ),
}));

vi.mock("@/components/launcher", () => ({
  LauncherPanel: () => <div data-testid="mock-launcher-panel">launcher</div>,
}));

vi.mock("@/components/system-check", () => ({
  SystemCheckDashboard: () => (
    <div data-testid="mock-system-check">
      <h2>系统自检 Dashboard</h2>
    </div>
  ),
  SYSTEM_PROBE_QUERY_KEY: ["system_probe", "run"] as const,
}));

vi.mock("@/components/sandbox/SandboxSettings", () => ({
  SandboxSettings: () => <div data-testid="mock-sandbox-settings">sandbox</div>,
}));

// Stub hooks pulled in by App.tsx that touch query layers we don't care about.
vi.mock("@/hooks/useProviderActions", () => ({
  useProviderActions: () => ({
    addProvider: async () => {},
    updateProvider: async () => {},
    switchProvider: async () => {},
    deleteProvider: async () => {},
    saveUsageScript: async () => {},
    setAsDefaultModel: async () => {},
  }),
}));

vi.mock("@/hooks/useOpenClaw", () => ({
  openclawKeys: {},
  useOpenClawHealth: () => ({ data: [] }),
}));

vi.mock("@/hooks/useHermes", () => ({
  hermesKeys: {},
  useOpenHermesWebUI: () => ({ data: null }),
}));

vi.mock("@/lib/api/hermes", () => ({
  hermesApi: {
    launchDashboard: async () => {},
  },
}));

vi.mock("@/hooks/useProxyStatus", () => ({
  useProxyStatus: () => ({
    isRunning: false,
    takeoverStatus: {},
    status: { active_targets: [] },
  }),
}));

vi.mock("@/hooks/useAutoCompact", () => ({
  useAutoCompact: () => false,
}));

vi.mock("@/hooks/useUsageCacheBridge", () => ({
  useUsageCacheBridge: () => {},
}));

vi.mock("@/lib/query/omo", () => ({
  useDisableCurrentOmo: () => ({ mutate: () => {} }),
  useDisableCurrentOmoSlim: () => ({ mutate: () => {} }),
}));

vi.mock("@/lib/api/env", () => ({
  checkAllEnvConflicts: async () => ({}),
  checkEnvConflicts: async () => [],
}));

vi.mock("@/lib/query", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/query")>("@/lib/query");
  return {
    ...actual,
    useProvidersQuery: () => ({
      data: { providers: {}, currentProviderId: "" },
      isLoading: false,
      refetch: async () => ({}),
    }),
    useSettingsQuery: () => ({
      data: {
        useAppWindowControls: false,
        visibleApps: {
          claude: true,
          "claude-desktop": false,
          codex: false,
          gemini: false,
          opencode: false,
          openclaw: false,
          hermes: false,
        },
        enableLocalProxy: false,
        enableFailoverToggle: false,
      },
    }),
  };
});

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    providersApi: {
      ...((actual as Record<string, unknown>).providersApi as object),
      onSwitched: async () => () => {},
      updateTrayMenu: async () => {},
    },
    settingsApi: (actual as Record<string, unknown>).settingsApi,
  };
});

// ---------------------------------------------------------------------------
// Imports that depend on the mocks above must be inline-required per test.
// ---------------------------------------------------------------------------

import {
  mockController,
  renderWithMockIPC,
  teardownMockIPC,
} from "@/lib/api/mock";

const user = userEvent.setup({
  pointerEventsCheck: PointerEventsCheckLevel.Never,
});

async function loadApp() {
  const mod = await import("@/App");
  return mod.default;
}

beforeEach(() => {
  // Clear the persisted view so onboarding logic doesn't get masked by prior
  // localStorage state from another test in the same worker.
  try {
    localStorage.removeItem("cc-switch-last-view");
    localStorage.removeItem("cc-switch-last-app");
  } catch {
    // ignore — jsdom always has localStorage but be defensive
  }
  // setupTests.ts globally disables the first-launch redirect so legacy
  // integration tests aren't affected. The redirect-specific test below
  // opts back in explicitly.
  if (typeof window !== "undefined") {
    (
      window as unknown as { __CC_DISABLE_FIRST_LAUNCH_REDIRECT__?: boolean }
    ).__CC_DISABLE_FIRST_LAUNCH_REDIRECT__ = true;
  }
});

afterEach(() => {
  teardownMockIPC();
  vi.clearAllMocks();
});

describe("App launcher view injection", () => {
  it("keeps the providers view as the initial view when onboarding is completed", async () => {
    const App = await loadApp();
    renderWithMockIPC("fully-configured", <App />);

    // CC Launcher logo (providers view header) should be visible. The Rocket
    // launcher entry button has the localized title "启动器".
    await waitFor(() => {
      expect(screen.getByText("CC Launcher")).toBeInTheDocument();
    });
    expect(screen.queryByTestId("mock-system-check")).not.toBeInTheDocument();
  });

  it("auto-redirects to launcherSystemCheck on first launch (onboarding not completed)", async () => {
    // Opt-in to the first-launch redirect for this specific test. The global
    // default (set by setupTests.ts) keeps it disabled so legacy integration
    // tests aren't affected.
    if (typeof window !== "undefined") {
      (
        window as unknown as { __CC_DISABLE_FIRST_LAUNCH_REDIRECT__?: boolean }
      ).__CC_DISABLE_FIRST_LAUNCH_REDIRECT__ = false;
    }

    const App = await loadApp();
    renderWithMockIPC("new-user", <App />);

    await waitFor(() => {
      expect(screen.getByTestId("mock-system-check")).toBeInTheDocument();
    });
    expect(screen.queryByText("CC Launcher")).not.toBeInTheDocument();
  });

  it("opens launcherHome when the 🚀 launcher entry button is clicked from providers view", async () => {
    const App = await loadApp();
    renderWithMockIPC("fully-configured", <App />);

    await waitFor(() => {
      expect(screen.getByText("CC Launcher")).toBeInTheDocument();
    });

    // The Rocket button is identified via its localized title attribute.
    const launcherEntry = await screen.findByTitle("启动器");
    await user.click(launcherEntry);

    await waitFor(() => {
      expect(screen.getByTestId("launcher-home-page")).toBeInTheDocument();
    });
  });

  it("navigates from launcherHome to launcherInstall when the Install step card is clicked", async () => {
    const App = await loadApp();
    renderWithMockIPC("fully-configured", <App />);

    await waitFor(() => {
      expect(screen.getByText("CC Launcher")).toBeInTheDocument();
    });

    await user.click(await screen.findByTitle("启动器"));
    await waitFor(() => {
      expect(screen.getByTestId("launcher-home-page")).toBeInTheDocument();
    });

    const installCard = await screen.findByTestId("launcher-home-step-install");
    await user.click(within(installCard).getByRole("button"));

    await waitFor(() => {
      expect(screen.getByTestId("mock-installer-wizard")).toBeInTheDocument();
    });
  });

  it("back-arrow from a launcher sub-view returns to launcherHome, then to providers", async () => {
    const App = await loadApp();
    renderWithMockIPC("fully-configured", <App />);

    await waitFor(() => {
      expect(screen.getByText("CC Launcher")).toBeInTheDocument();
    });

    await user.click(await screen.findByTitle("启动器"));
    await waitFor(() => {
      expect(screen.getByTestId("launcher-home-page")).toBeInTheDocument();
    });

    // Drill into Install sub-view via the card.
    const installCard = await screen.findByTestId("launcher-home-step-install");
    await user.click(within(installCard).getByRole("button"));
    await waitFor(() => {
      expect(screen.getByTestId("mock-installer-wizard")).toBeInTheDocument();
    });

    // The header now has an ArrowLeft icon button (no accessible name; we look
    // it up by its DOM position — the only icon-only button to the left of the
    // page title).
    const backButtons = screen
      .getAllByRole("button")
      .filter((btn) => btn.querySelector("svg.lucide-arrow-left"));
    expect(backButtons.length).toBeGreaterThan(0);
    await user.click(backButtons[0]);

    await waitFor(() => {
      expect(screen.getByTestId("launcher-home-page")).toBeInTheDocument();
    });

    // From launcherHome back-arrow returns to providers.
    const backButtons2 = screen
      .getAllByRole("button")
      .filter((btn) => btn.querySelector("svg.lucide-arrow-left"));
    expect(backButtons2.length).toBeGreaterThan(0);
    await user.click(backButtons2[0]);

    await waitFor(() => {
      expect(screen.getByText("CC Launcher")).toBeInTheDocument();
    });
  });
});

// ---------------------------------------------------------------------------
// Issue 3: onboarding lock — when onboarding.get_state().completed === false,
// the user is force-funneled through the 5-step launcher sequence and cannot
// exit until they finish the final step.
// ---------------------------------------------------------------------------

describe("Onboarding lock (first-launch flow enforcement)", () => {
  beforeEach(() => {
    // Enable the first-launch redirect for this describe block so the App
    // actually mounts in the locked launcherSystemCheck view.
    if (typeof window !== "undefined") {
      (
        window as unknown as { __CC_DISABLE_FIRST_LAUNCH_REDIRECT__?: boolean }
      ).__CC_DISABLE_FIRST_LAUNCH_REDIRECT__ = false;
    }
  });

  it("does not render the Rocket header entry button while onboarding is locked", async () => {
    const App = await loadApp();
    renderWithMockIPC("new-user", <App />);

    // First-launch redirect lands on launcherSystemCheck.
    await waitFor(() => {
      expect(screen.getByTestId("mock-system-check")).toBeInTheDocument();
    });

    // Rocket entry is only ever rendered inside the providers view (which we
    // can't reach while locked). It must not be present here.
    expect(screen.queryByTestId("header-rocket-entry")).not.toBeInTheDocument();
  });

  it("hides the header back-arrow on launcherSystemCheck while onboarding is locked", async () => {
    const App = await loadApp();
    renderWithMockIPC("new-user", <App />);

    await waitFor(() => {
      expect(screen.getByTestId("mock-system-check")).toBeInTheDocument();
    });

    // No ArrowLeft button — the user cannot escape the launcher sequence.
    const backButtons = screen
      .queryAllByRole("button")
      .filter((btn) => btn.querySelector("svg.lucide-arrow-left"));
    expect(backButtons).toHaveLength(0);

    // The system-check view is still rendered (i.e. we did not navigate away).
    expect(screen.getByTestId("mock-system-check")).toBeInTheDocument();
  });

  it("advances through all 5 steps and on the final 'complete' button enters the providers view", async () => {
    const App = await loadApp();
    renderWithMockIPC("new-user", <App />);

    // Step 1 / 5 — system check.
    await waitFor(() => {
      expect(screen.getByTestId("mock-system-check")).toBeInTheDocument();
    });
    await user.click(
      screen.getByTestId("launcher-step-footer-next-launcherSystemCheck"),
    );

    // Step 2 / 5 — sandbox.
    await waitFor(() => {
      expect(screen.getByTestId("mock-sandbox-settings")).toBeInTheDocument();
    });
    await user.click(
      screen.getByTestId("launcher-step-footer-next-launcherSandbox"),
    );

    // Step 3 / 5 — install.
    await waitFor(() => {
      expect(screen.getByTestId("mock-installer-wizard")).toBeInTheDocument();
    });
    await user.click(
      screen.getByTestId("launcher-step-footer-next-launcherInstall"),
    );

    // Step 4 / 5 — profile.
    await waitFor(() => {
      expect(screen.getByTestId("mock-profile-manager")).toBeInTheDocument();
    });
    await user.click(
      screen.getByTestId("launcher-step-footer-next-launcherProfile"),
    );

    // Step 5 / 5 — launch. The footer should now show the "complete" button,
    // NOT the regular "next" button.
    await waitFor(() => {
      expect(screen.getByTestId("mock-launcher-panel")).toBeInTheDocument();
    });
    expect(
      screen.queryByTestId("launcher-step-footer-next-launcherLaunch"),
    ).not.toBeInTheDocument();
    const completeBtn = screen.getByTestId("launcher-step-footer-complete");
    await user.click(completeBtn);

    // Final assertion — providers view is rendered (CC Launcher logo) and the
    // launcher sub-views are gone.
    await waitFor(() => {
      expect(screen.getByText("CC Launcher")).toBeInTheDocument();
    });
    expect(screen.queryByTestId("mock-launcher-panel")).not.toBeInTheDocument();
  });

  it("after onboarding is complete, clicking Rocket re-enters launcherHome and the back-arrow returns to providers", async () => {
    // This test uses the 'fully-configured' scenario which has
    // onboarding.completed === true from the start. The disabled-redirect
    // flag is still on by default, so providers is the initial view.
    if (typeof window !== "undefined") {
      (
        window as unknown as { __CC_DISABLE_FIRST_LAUNCH_REDIRECT__?: boolean }
      ).__CC_DISABLE_FIRST_LAUNCH_REDIRECT__ = true;
    }

    const App = await loadApp();
    renderWithMockIPC("fully-configured", <App />);

    await waitFor(() => {
      expect(screen.getByText("CC Launcher")).toBeInTheDocument();
    });

    // Rocket is visible.
    const rocket = await screen.findByTestId("header-rocket-entry");
    await user.click(rocket);

    await waitFor(() => {
      expect(screen.getByTestId("launcher-home-page")).toBeInTheDocument();
    });

    // Back-arrow on launcherHome returns to providers.
    const backButtons = screen
      .getAllByRole("button")
      .filter((btn) => btn.querySelector("svg.lucide-arrow-left"));
    expect(backButtons.length).toBeGreaterThan(0);
    await user.click(backButtons[0]);

    await waitFor(() => {
      expect(screen.getByText("CC Launcher")).toBeInTheDocument();
    });
  });
});

// Silence "unused" lints on imports kept for future expansion of these tests.
void mockController;
void act;
