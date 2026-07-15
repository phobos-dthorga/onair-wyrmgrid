<script lang="ts">
  import { onMount } from "svelte";
  import AtlasMap from "$lib/atlas/AtlasMap.svelte";
  import { atlasPreviewFbos, atlasPreviewFleet } from "$lib/atlas/sample";
  import type {
    AircraftSummary,
    AtlasFlightRoute,
    CompanyDataSyncResult,
    DataSyncTrigger,
    FboSnapshotView,
    FboSummary,
    FleetSnapshot,
    FleetSnapshotView,
    JobSnapshotView,
  } from "$lib/atlas/types";
  import WyrmChart from "$lib/charts/WyrmChart.svelte";
  import { foundationChart } from "$lib/charts/sample";
  import {
    invokeDesktop,
    isDesktopRuntime,
    operationErrorMessage,
  } from "$lib/desktop/client";
  import {
    clearDiagnosticLog,
    loadDiagnosticLog,
  } from "$lib/diagnostics/client";
  import DiagnosticsDialog from "$lib/diagnostics/DiagnosticsDialog.svelte";
  import {
    emptyDiagnosticLog,
    type DiagnosticLogView,
  } from "$lib/diagnostics/types";
  import {
    choosePortableBackupDestination,
    choosePortableBackupSource,
    createPortableBackup,
    loadDataProtectionStatus,
    preparePortableRestore,
  } from "$lib/data-protection/client";
  import DataProtectionDialog from "$lib/data-protection/DataProtectionDialog.svelte";
  import { browserDataProtectionStatus } from "$lib/data-protection/sample";
  import type { DataProtectionStatus } from "$lib/data-protection/types";
  import {
    clearDispatchPlan,
    importLatestSimBriefPlan,
    loadDispatchStatus,
    loadSimBriefAccountPreference,
    refreshDispatchWeather,
    selectDispatchJob,
  } from "$lib/dispatch/client";
  import DispatchWorkspace from "$lib/dispatch/DispatchWorkspace.svelte";
  import {
    dispatchPreviewEmpty,
    dispatchPreviewReady,
  } from "$lib/dispatch/sample";
  import type {
    DispatchStatus,
    SimBriefAccountPreference,
    SimBriefReferenceKind,
  } from "$lib/dispatch/types";
  import JobsWorkspace from "$lib/jobs/JobsWorkspace.svelte";
  import LaunchScreen from "$lib/launch/LaunchScreen.svelte";
  import {
    remainingLaunchDisplayTime,
    shouldRenderLaunchArtwork,
    viewportPresentation,
    type ViewportPresentation,
  } from "$lib/launch/presentation";
  import {
    defaultStartupOptions,
    loadStartupOptions,
    type StartupOptions,
  } from "$lib/launch/startup";
  import ForgeDialog from "$lib/forge/ForgeDialog.svelte";
  import {
    approvePluginPermissions,
    loadPluginHost,
    revokePluginPermissions,
    startPlugin,
    stopPlugin,
  } from "$lib/forge/client";
  import {
    forgePreviewApproved,
    forgePreviewRunning,
    forgePreviewStopped,
  } from "$lib/forge/sample";
  import type {
    AuthorizationGrantLifetime,
    PluginHostView,
  } from "$lib/forge/types";
  import LegalDialog from "$lib/legal/LegalDialog.svelte";
  import OpenSourceLicencesDialog from "$lib/legal/OpenSourceLicencesDialog.svelte";
  import {
    CURRENT_PRIVACY_NOTICE_VERSION,
    CURRENT_TERMS_VERSION,
    acknowledgeLegal,
    loadLegalStatus,
    updateTelemetryPreference,
    type LegalStatus,
  } from "$lib/legal/client";
  import {
    fboGrowthChart,
    fleetCompositionChart,
    fleetGrowthChart,
  } from "$lib/hoard/charts";
  import HoardTimelineDialog from "$lib/hoard/HoardTimelineDialog.svelte";
  import {
    hoardPreviewTimeline,
    previewHistoricalCompanyData,
  } from "$lib/hoard/sample";
  import type {
    HistoricalCompanyDataView,
    HoardTimelineIndex,
    TimelineMode,
  } from "$lib/hoard/types";
  import LanguageDialog from "$lib/i18n/LanguageDialog.svelte";
  import {
    browserLanguageStatus,
    importLanguagePack,
    loadLanguageStatus,
    selectLanguagePack,
  } from "$lib/i18n/client";
  import { applyLanguage, translate, translation } from "$lib/i18n/runtime";
  import type { LanguageStatus } from "$lib/i18n/types";
  import { configureClientTelemetry } from "$lib/observability/client";
  import ConnectionDialog from "$lib/onair/ConnectionDialog.svelte";
  import { autoConnectOnAirIfEnabled } from "$lib/onair/client";
  import {
    disconnectedStatus,
    type OnAirConnectionStatus,
  } from "$lib/onair/types";
  import {
    loadSimulatorBridge,
    loadSimulatorPreferences,
    loadSimulatorRecording,
    loadSimulatorRecordingDebrief,
    loadSimulatorRecordingSession,
    saveSimulatorPreferences,
    saveSimulatorRecordingPreferences,
    startSimulatorProvider,
    startSimulatorRecording,
    stopSimulatorProvider,
    stopSimulatorRecording,
    deleteSimulatorRecording,
    deleteAllSimulatorRecordings,
    exportSimulatorRecording,
    pinSimulatorRecording,
  } from "$lib/simulator/client";
  import SimulatorDialog from "$lib/simulator/SimulatorDialog.svelte";
  import {
    emptySimulatorBridge,
    emptySimulatorRecording,
    defaultSimulatorPreferences,
    type SimulatorBridgeView,
    type SimulatorPreferences,
    type SimulatorRecordingPreferences,
    type SimulatorRecordingView,
    type SimulatorSessionDebrief,
    type SimulatorSessionView,
  } from "$lib/simulator/types";
  import {
    simulatorRecordingPreview,
    simulatorRecordingDebriefPreview,
    simulatorRecordingSessionPreview,
  } from "$lib/simulator/sample";
  import {
    loadDisplayPreferences,
    saveDisplayPreferences,
  } from "$lib/settings/client";
  import SettingsDialog from "$lib/settings/SettingsDialog.svelte";
  import {
    aviationDisplayPreferences,
    type DisplayPreferences,
  } from "$lib/settings/types";
  import {
    closedDialogNavigation,
    enterDialogSurface,
    isDialogSurface,
    leaveDialogSurface,
    openDialogNavigation,
    type DialogNavigation,
  } from "$lib/navigation/dialogStack";
  import SecurityCentreDialog from "$lib/security/SecurityCentreDialog.svelte";
  import { loadSecurityCentre } from "$lib/security/client";
  import {
    securityPreviewEmpty,
    securityPreviewGranted,
    securityPreviewRevoked,
  } from "$lib/security/sample";
  import type { SecurityCentreStatus } from "$lib/security/types";
  import ThemeDialog from "$lib/theme/ThemeDialog.svelte";
  import {
    browserThemeStatus,
    importTheme,
    loadThemeStatus,
    selectTheme,
  } from "$lib/theme/client";
  import { applyTheme } from "$lib/theme/runtime";
  import type { ThemeStatus } from "$lib/theme/types";

  type PlatformStatus = {
    application: string;
    version: string;
    plugin_api_version: number;
    mode: string;
  };

  type FleetLoadState = "idle" | "loading" | "ready" | "error";
  type LegalLoadState = "loading" | "ready" | "error";
  type Workspace = "atlas" | "jobs" | "dispatch";
  type AppDialogSurface =
    | "connection"
    | "diagnostics"
    | "simulator"
    | "settings"
    | "theme"
    | "language"
    | "privacy"
    | "security"
    | "data_protection"
    | "licenses"
    | "hoard"
    | "forge";

  const AUTOMATIC_SYNC_STORAGE_KEY = "wyrmgrid.atlas.automatic-sync-minutes";
  const AUTOMATIC_SYNC_OPTIONS = [0, 15, 30, 60, 120] as const;
  const launchStartedAt = Date.now();

  let status = $state<PlatformStatus>({
    application: "OnAir WyrmGrid",
    version: "0.1.0",
    plugin_api_version: 1,
    mode: "browser preview",
  });
  let startupOptions = $state<StartupOptions>(defaultStartupOptions);
  let startupOptionsLoaded = $state(false);
  let viewportMode = $state<ViewportPresentation>("standard");
  let connection = $state<OnAirConnectionStatus>(disconnectedStatus);
  let activeWorkspace = $state<Workspace>("atlas");
  let dispatchStatus = $state<DispatchStatus>(dispatchPreviewEmpty);
  let dispatchBusy = $state(false);
  let dispatchError = $state("");
  let simbriefAccountPreference = $state<SimBriefAccountPreference>();
  let fleetView = $state<FleetSnapshotView | null>(null);
  let fboView = $state<FboSnapshotView | null>(null);
  let jobView = $state<JobSnapshotView | null>(null);
  let fleetLoadState = $state<FleetLoadState>("idle");
  let fleetError = $state("");
  let fleetVisible = $state(true);
  let fboVisible = $state(true);
  let selectedAircraftId = $state<string | null>(null);
  let selectedFboId = $state<string | null>(null);
  let automaticSyncMinutes = $state(30);
  let legalStatus = $state<LegalStatus>({
    terms_version: CURRENT_TERMS_VERSION,
    privacy_notice_version: CURRENT_PRIVACY_NOTICE_VERSION,
    acknowledged: false,
    telemetry_enabled: false,
  });
  let legalLoadState = $state<LegalLoadState>("loading");
  let legalError = $state("");
  let legalBusy = $state(false);
  let legalTelemetryDraft = $state(false);
  let showLegalDialog = $state(false);
  let themeStatus = $state<ThemeStatus>(browserThemeStatus);
  let themeBusy = $state(false);
  let themeError = $state("");
  let languageStatus = $state<LanguageStatus>(browserLanguageStatus);
  let languageBusy = $state(false);
  let languageError = $state("");
  let diagnosticLog = $state<DiagnosticLogView>(emptyDiagnosticLog);
  let diagnosticsBusy = $state(false);
  let diagnosticsError = $state("");
  let simulatorBridge = $state<SimulatorBridgeView>(emptySimulatorBridge);
  let simulatorBusy = $state(false);
  let simulatorError = $state("");
  let simulatorPreferences = $state<SimulatorPreferences>(
    defaultSimulatorPreferences,
  );
  let simulatorRecording = $state<SimulatorRecordingView>(
    emptySimulatorRecording,
  );
  let simulatorRecordingSession = $state<SimulatorSessionView>();
  let simulatorRecordingDebrief = $state<SimulatorSessionDebrief>();
  let atlasFlightRoute = $state<AtlasFlightRoute>();
  let simulatorRecordingBusy = $state(false);
  let displayPreferences = $state<DisplayPreferences>(
    aviationDisplayPreferences,
  );
  let dialogNavigation = $state<DialogNavigation<AppDialogSurface>>(
    closedDialogNavigation<AppDialogSurface>(),
  );
  let settingsBusy = $state(false);
  let settingsError = $state("");
  let dataProtectionStatus = $state<DataProtectionStatus>(
    browserDataProtectionStatus,
  );
  let dataProtectionLoaded = $state(false);
  let dataProtectionBusy = $state(false);
  let dataProtectionError = $state("");
  let dataProtectionSuccess = $state("");
  let securityCentre = $state<SecurityCentreStatus>(securityPreviewEmpty);
  let securityCentreLoaded = $state(false);
  let securityBusy = $state(false);
  let securityError = $state("");
  let timeline = $state<HoardTimelineIndex>({
    company: null,
    observation_times: [],
    fleet_history: [],
    fbo_history: [],
    current_fleet_composition: [],
  });
  let timelineMode = $state<TimelineMode>("live");
  let historicalData = $state<HistoricalCompanyDataView | null>(null);
  let timelineCursor = $state(0);
  let timelineBusy = $state(false);
  let timelineError = $state("");
  let pluginHost = $state<PluginHostView>(forgePreviewStopped);
  let pluginLayersVisible = $state(true);
  let pluginBusy = $state(false);
  let pluginError = $state("");
  let workspaceInitialized = false;

  const showSettingsDialog = $derived(
    isDialogSurface(dialogNavigation, "settings"),
  );
  const showThemeDialog = $derived(isDialogSurface(dialogNavigation, "theme"));
  const showLanguageDialog = $derived(
    isDialogSurface(dialogNavigation, "language"),
  );
  const showDataProtection = $derived(
    isDialogSurface(dialogNavigation, "data_protection"),
  );
  const showOpenSourceLicences = $derived(
    isDialogSurface(dialogNavigation, "licenses"),
  );
  const showSecurityCentre = $derived(
    isDialogSurface(dialogNavigation, "security"),
  );
  const showSettingsPrivacy = $derived(
    isDialogSurface(dialogNavigation, "privacy"),
  );
  const showConnectionDialog = $derived(
    isDialogSurface(dialogNavigation, "connection"),
  );
  const showDiagnosticsDialog = $derived(
    isDialogSurface(dialogNavigation, "diagnostics"),
  );
  const showSimulatorDialog = $derived(
    isDialogSurface(dialogNavigation, "simulator"),
  );
  const showTimelineDialog = $derived(
    isDialogSurface(dialogNavigation, "hoard"),
  );
  const showForgeDialog = $derived(isDialogSurface(dialogNavigation, "forge"));

  const activeFleetView = $derived(
    timelineMode === "historical" ? (historicalData?.fleet ?? null) : fleetView,
  );
  const activeFboView = $derived(
    timelineMode === "historical" ? (historicalData?.fbos ?? null) : fboView,
  );
  const fleetSnapshot = $derived<FleetSnapshot | null>(
    activeFleetView?.snapshot ?? null,
  );
  const aircraft = $derived(fleetSnapshot?.value ?? []);
  const fbos = $derived(activeFboView?.snapshot.value ?? []);
  const jobs = $derived(jobView?.snapshot.value.jobs ?? []);
  const plottedAircraftCount = $derived(
    aircraft.filter((item) => item.location).length,
  );
  const plottedFboCount = $derived(
    fbos.filter((item) => item.airport?.location).length,
  );
  const selectedAircraft = $derived(
    aircraft.find((item) => item.id === selectedAircraftId) ?? null,
  );
  const selectedFbo = $derived(
    fbos.find((item) => item.id === selectedFboId) ?? null,
  );
  const atlasAvailability = $derived(
    activeFleetView?.availability ?? activeFboView?.availability,
  );
  const launchArtworkEnabled = $derived(
    shouldRenderLaunchArtwork(
      startupOptionsLoaded,
      startupOptions.no_launch_art,
    ),
  );
  const atlasStorage = $derived(
    activeFleetView?.storage ?? activeFboView?.storage,
  );
  const atlasCompany = $derived(
    activeFleetView?.company ?? activeFboView?.company,
  );
  const atlasObservedAt = $derived(
    fleetSnapshot?.provenance.observed_at ??
      activeFboView?.snapshot.provenance.observed_at,
  );
  const fleetSourceLabel = $derived(
    atlasAvailability === "preview"
      ? "Illustrative preview"
      : atlasAvailability === "live"
        ? "OnAir fact"
        : "Cached OnAir fact",
  );
  const fleetAvailabilityLabel = $derived(
    atlasAvailability === "live"
      ? "Live"
      : atlasAvailability === "cached"
        ? "Cached"
        : atlasAvailability === "offline"
          ? "Offline"
          : atlasAvailability === "preview"
            ? "Preview"
            : "Unavailable",
  );
  const fleetStorageLabel = $derived(
    atlasStorage === "hoard"
      ? "Hoard snapshot"
      : atlasStorage === "memory_only"
        ? "Memory only"
        : atlasStorage === "preview"
          ? "Preview data"
          : "No snapshot",
  );
  const fboAvailabilityLabel = $derived(
    activeFboView?.availability === "live"
      ? "Live"
      : activeFboView?.availability === "cached"
        ? "Cached"
        : activeFboView?.availability === "offline"
          ? "Offline"
          : activeFboView?.availability === "preview"
            ? "Preview"
            : "Unavailable",
  );
  const fleetResourceAvailabilityLabel = $derived(
    activeFleetView?.availability === "live"
      ? "Live"
      : activeFleetView?.availability === "cached"
        ? "Cached"
        : activeFleetView?.availability === "offline"
          ? "Offline"
          : activeFleetView?.availability === "preview"
            ? "Preview"
            : "Unavailable",
  );
  const fboSourceLabel = $derived(
    activeFboView?.availability === "preview"
      ? "Illustrative preview"
      : activeFboView?.availability === "live"
        ? "OnAir fact"
        : "Cached OnAir fact",
  );
  const timelineGrowthChart = $derived(
    fleetGrowthChart(timeline.fleet_history),
  );
  const timelineFboGrowthChart = $derived(fboGrowthChart(timeline.fbo_history));
  const timelineComposition = $derived(
    timelineMode === "historical"
      ? (historicalData?.fleet_composition ?? [])
      : timeline.current_fleet_composition,
  );
  const timelineCompositionObservedAt = $derived(
    timelineMode === "historical"
      ? historicalData?.fleet?.snapshot.provenance.observed_at
      : fleetView?.snapshot.provenance.observed_at,
  );
  const timelineFleetCompositionChart = $derived(
    fleetCompositionChart(timelineComposition, timelineCompositionObservedAt),
  );
  const pluginPointCount = $derived(
    pluginHost.layers.reduce(
      (total, published) => total + published.layer.points.length,
      0,
    ),
  );
  const pluginProcessActive = $derived(
    pluginHost.plugins.some((plugin) =>
      ["starting", "running", "stopping"].includes(plugin.state),
    ),
  );
  const simulatorProcessActive = $derived(
    simulatorBridge.providers.some((provider) =>
      ["starting", "running", "stopping"].includes(provider.process_state),
    ),
  );
  const layers = $derived([
    {
      id: "fleet",
      name: "Fleet",
      count: plottedAircraftCount,
      active: fleetVisible,
      available: true,
    },
    {
      id: "fbos",
      name: "FBO network",
      count: plottedFboCount,
      active: fboVisible,
      available: true,
    },
    {
      id: "jobs",
      name: "Jobs",
      count: jobs.length,
      active: jobs.length > 0,
      available: true,
    },
    {
      id: "maintenance",
      name: "Maintenance",
      count: 0,
      active: false,
      available: false,
    },
    {
      id: "plugins",
      name: "Plugin layers",
      count: pluginPointCount,
      active: pluginLayersVisible && pluginPointCount > 0,
      available: pluginPointCount > 0,
    },
  ]);

  function safeError(error: unknown): string {
    return operationErrorMessage(
      error,
      "WyrmGrid could not synchronize company data.",
    );
  }

  function formatObservedAt(value: string | undefined): string {
    if (!value) return "No fleet observation yet";
    const observed = new Date(value);
    return Number.isNaN(observed.getTime())
      ? "Observation time unavailable"
      : `Observed ${observed.toLocaleString()}`;
  }

  function countedLabel(
    count: number,
    singular: string,
    plural = `${singular}s`,
  ): string {
    return `${count} ${count === 1 ? singular : plural}`;
  }

  function displayRegistration(item: AircraftSummary): string {
    return item.registration ?? "Unknown registration";
  }

  function formatCoordinates(item: AircraftSummary): string {
    if (!item.location) return "Location unavailable";
    return `${item.location.latitude.toFixed(4)}, ${item.location.longitude.toFixed(4)}`;
  }

  function formatFboCoordinates(item: FboSummary): string {
    if (!item.airport?.location) return "Location unavailable";
    return `${item.airport.location.latitude.toFixed(4)}, ${item.airport.location.longitude.toFixed(4)}`;
  }

  function acceptFleetView(view: FleetSnapshotView): void {
    fleetView = view;
    if (
      selectedAircraftId &&
      !view.snapshot.value.some((item) => item.id === selectedAircraftId)
    ) {
      selectedAircraftId = null;
    }
    if (dispatchStatus.snapshot) void refreshDispatchStatus();
  }

  function acceptFboView(view: FboSnapshotView): void {
    fboView = view;
    if (
      selectedFboId &&
      !view.snapshot.value.some((item) => item.id === selectedFboId)
    ) {
      selectedFboId = null;
    }
  }

  function returnToPresent(): void {
    timelineMode = "live";
    historicalData = null;
    timelineError = "";
    timelineCursor = Math.max(0, timeline.observation_times.length - 1);
    selectedAircraftId = null;
    selectedFboId = null;
    void refreshTimeline();
  }

  async function refreshTimeline(): Promise<void> {
    if (!isDesktopRuntime()) {
      timeline = hoardPreviewTimeline;
      if (timelineMode === "live") {
        timelineCursor = Math.max(0, timeline.observation_times.length - 1);
      }
      return;
    }
    try {
      const refreshed = await invokeDesktop<HoardTimelineIndex>(
        "onair_hoard_timeline",
      );
      const selectedIndex = historicalData
        ? refreshed.observation_times.indexOf(historicalData.selected_at)
        : -1;
      if (
        timelineMode === "historical" &&
        historicalData &&
        selectedIndex < 0
      ) {
        return;
      }
      timeline = refreshed;
      if (timelineMode === "live") {
        timelineCursor = Math.max(0, refreshed.observation_times.length - 1);
      } else if (selectedIndex >= 0) {
        timelineCursor = selectedIndex;
      }
    } catch (error) {
      timelineError = operationErrorMessage(
        error,
        "WyrmGrid could not read the Hoard timeline.",
      );
    }
  }

  function openHoardTimeline(): void {
    timelineError = "";
    simulatorError = "";
    openRootDialog("hoard");
    void Promise.all([refreshTimeline(), refreshSimulatorRecording()]);
  }

  async function refreshDiagnostics(): Promise<void> {
    if (!isDesktopRuntime() || diagnosticsBusy) return;
    diagnosticsBusy = true;
    diagnosticsError = "";
    try {
      diagnosticLog = await loadDiagnosticLog();
    } catch (error) {
      diagnosticsError = operationErrorMessage(
        error,
        "WyrmGrid could not read the local diagnostic log.",
      );
    } finally {
      diagnosticsBusy = false;
    }
  }

  function openDiagnostics(): void {
    diagnosticsError = "";
    openRootDialog("diagnostics");
    void refreshDiagnostics();
  }

  async function clearDiagnostics(): Promise<void> {
    if (!isDesktopRuntime() || diagnosticsBusy) return;
    diagnosticsBusy = true;
    diagnosticsError = "";
    try {
      diagnosticLog = await clearDiagnosticLog();
    } catch (error) {
      diagnosticsError = operationErrorMessage(
        error,
        "WyrmGrid could not clear the local diagnostic log.",
      );
    } finally {
      diagnosticsBusy = false;
    }
  }

  async function refreshSimulatorBridge(): Promise<void> {
    if (!isDesktopRuntime()) {
      simulatorBridge = emptySimulatorBridge;
      return;
    }
    try {
      simulatorBridge = await loadSimulatorBridge();
    } catch (error) {
      simulatorError = operationErrorMessage(
        error,
        "WyrmGrid could not read simulator provider status.",
      );
    }
  }

  async function refreshSimulatorRecording(): Promise<void> {
    if (!isDesktopRuntime()) {
      simulatorRecording = simulatorRecordingPreview;
      simulatorRecordingSession = simulatorRecordingSessionPreview;
      simulatorRecordingDebrief = simulatorRecordingDebriefPreview;
      return;
    }
    try {
      simulatorRecording = await loadSimulatorRecording();
      const currentSessionId = simulatorRecordingSession?.session.id;
      const selectedId =
        simulatorRecording.active_session_id ??
        (currentSessionId &&
        simulatorRecording.sessions.some(
          (session) => session.id === currentSessionId,
        )
          ? currentSessionId
          : undefined) ??
        simulatorRecording.sessions[0]?.id;
      if (selectedId) {
        [simulatorRecordingSession, simulatorRecordingDebrief] =
          await Promise.all([
            loadSimulatorRecordingSession(selectedId),
            loadSimulatorRecordingDebrief(selectedId),
          ]);
      } else {
        simulatorRecordingSession = undefined;
        simulatorRecordingDebrief = undefined;
      }
    } catch (error) {
      simulatorError = operationErrorMessage(
        error,
        "WyrmGrid could not read local simulator recordings.",
      );
    }
  }

  function openSimulator(): void {
    simulatorError = "";
    openRootDialog("simulator");
    void Promise.all([refreshSimulatorBridge(), refreshSimulatorRecording()]);
  }

  async function runRecordingAction(
    action: "start" | "stop" | "delete" | "delete_all",
    sessionId?: string,
  ): Promise<void> {
    if (!isDesktopRuntime() || simulatorRecordingBusy) return;
    simulatorRecordingBusy = true;
    simulatorError = "";
    try {
      if (action === "start")
        simulatorRecording = await startSimulatorRecording();
      if (action === "stop")
        simulatorRecording = await stopSimulatorRecording();
      if (action === "delete" && sessionId)
        simulatorRecording = await deleteSimulatorRecording(sessionId);
      if (action === "delete_all")
        simulatorRecording = await deleteAllSimulatorRecordings();
      simulatorRecordingSession = undefined;
      simulatorRecordingDebrief = undefined;
      await refreshSimulatorRecording();
    } catch (error) {
      simulatorError = operationErrorMessage(
        error,
        "WyrmGrid could not complete that recording action.",
      );
      await refreshSimulatorRecording();
    } finally {
      simulatorRecordingBusy = false;
    }
  }

  async function selectRecordingSession(sessionId: string): Promise<void> {
    if (!isDesktopRuntime() || simulatorRecordingBusy) return;
    simulatorRecordingBusy = true;
    simulatorError = "";
    try {
      [simulatorRecordingSession, simulatorRecordingDebrief] =
        await Promise.all([
          loadSimulatorRecordingSession(sessionId),
          loadSimulatorRecordingDebrief(sessionId),
        ]);
    } catch (error) {
      simulatorError = operationErrorMessage(
        error,
        "WyrmGrid could not open that local simulator recording.",
      );
    } finally {
      simulatorRecordingBusy = false;
    }
  }

  async function pageRecordingSession(
    sessionId: string,
    sampleOffset: number,
  ): Promise<void> {
    if (!isDesktopRuntime() || simulatorRecordingBusy) return;
    simulatorRecordingBusy = true;
    simulatorError = "";
    try {
      simulatorRecordingSession = await loadSimulatorRecordingSession(
        sessionId,
        sampleOffset,
      );
    } catch (error) {
      simulatorError = operationErrorMessage(
        error,
        "WyrmGrid could not open that recording window.",
      );
    } finally {
      simulatorRecordingBusy = false;
    }
  }

  async function setRecordingPinned(
    sessionId: string,
    pinned: boolean,
  ): Promise<void> {
    if (!isDesktopRuntime() || simulatorRecordingBusy) return;
    simulatorRecordingBusy = true;
    simulatorError = "";
    try {
      simulatorRecording = await pinSimulatorRecording(sessionId, pinned);
      [simulatorRecordingSession, simulatorRecordingDebrief] =
        await Promise.all([
          loadSimulatorRecordingSession(sessionId),
          loadSimulatorRecordingDebrief(sessionId),
        ]);
    } catch (error) {
      simulatorError = operationErrorMessage(
        error,
        "WyrmGrid could not update that recording.",
      );
    } finally {
      simulatorRecordingBusy = false;
    }
  }

  async function exportRecording(
    sessionId: string,
    format: "json" | "csv",
  ): Promise<void> {
    if (!isDesktopRuntime() || simulatorRecordingBusy) return;
    simulatorRecordingBusy = true;
    simulatorError = "";
    try {
      const exported = await exportSimulatorRecording(sessionId, format);
      const url = URL.createObjectURL(
        new Blob([exported.content], { type: exported.media_type }),
      );
      const link = document.createElement("a");
      link.href = url;
      link.download = exported.filename;
      link.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      simulatorError = operationErrorMessage(
        error,
        "WyrmGrid could not export that recording.",
      );
    } finally {
      simulatorRecordingBusy = false;
    }
  }

  function openRecordingRouteInAtlas(route: AtlasFlightRoute): void {
    atlasFlightRoute = route;
    selectedAircraftId = null;
    selectedFboId = null;
    activeWorkspace = "atlas";
    dialogNavigation = closedDialogNavigation<AppDialogSurface>();
  }

  async function runSimulatorAction(
    action: "start" | "stop",
    providerId: string,
  ): Promise<void> {
    if (!isDesktopRuntime() || simulatorBusy) return;
    simulatorBusy = true;
    simulatorError = "";
    try {
      simulatorBridge =
        action === "start"
          ? await startSimulatorProvider(providerId)
          : await stopSimulatorProvider(providerId);
      if (action === "start") {
        simulatorPreferences = {
          ...simulatorPreferences,
          selected_provider_id: providerId,
        };
      }
    } catch (error) {
      simulatorError = operationErrorMessage(
        error,
        "WyrmGrid could not complete that simulator provider action.",
      );
      await refreshSimulatorBridge();
    } finally {
      simulatorBusy = false;
    }
  }

  async function viewHistoricalMoment(): Promise<void> {
    const selectedAt = timeline.observation_times[timelineCursor];
    if (!selectedAt || timelineBusy) return;
    timelineBusy = true;
    timelineError = "";
    try {
      historicalData = isDesktopRuntime()
        ? await invokeDesktop<HistoricalCompanyDataView>(
            "onair_historical_company_data",
            { selectedAt },
          )
        : previewHistoricalCompanyData(selectedAt);
      timelineMode = "historical";
      selectedAircraftId = null;
      selectedFboId = null;
    } catch (error) {
      timelineError = operationErrorMessage(
        error,
        "WyrmGrid could not open that retained observation.",
      );
    } finally {
      timelineBusy = false;
    }
  }

  async function synchronizeCompanyData(
    trigger: DataSyncTrigger,
  ): Promise<void> {
    if (!connection.connected || fleetLoadState === "loading") return;
    fleetLoadState = "loading";
    fleetError = "";

    try {
      const result = await invokeDesktop<CompanyDataSyncResult>(
        "synchronize_onair_company_data",
        { trigger },
      );
      if (result.disposition === "quietly_ignored") {
        fleetLoadState = fleetView || fboView ? "ready" : "idle";
        await refreshTimeline();
        return;
      }

      if (result.fleet) acceptFleetView(result.fleet);
      if (result.fbos) acceptFboView(result.fbos);
      if (result.jobs) jobView = result.jobs;
      if (result.failures.length > 0) {
        fleetError = result.failures
          .map((failure) => failure.message)
          .join(" ");
        fleetLoadState = "error";
      } else {
        fleetLoadState = "ready";
      }
      await refreshTimeline();
    } catch (error) {
      fleetError = safeError(error);
      try {
        const retained = await invokeDesktop<FleetSnapshotView | null>(
          "onair_fleet_snapshot",
        );
        if (retained) acceptFleetView(retained);
        const retainedFbos = await invokeDesktop<FboSnapshotView | null>(
          "onair_fbo_snapshot",
        );
        if (retainedFbos) acceptFboView(retainedFbos);
        const retainedJobs = await invokeDesktop<JobSnapshotView | null>(
          "onair_job_snapshot",
        );
        if (retainedJobs) jobView = retainedJobs;
      } catch {
        // Keep the existing presentation state when Hoard cannot be read.
      }
      fleetLoadState = "error";
    }
  }

  async function restoreCompanySnapshots(
    synchronizeAfterRestore: boolean,
  ): Promise<void> {
    try {
      const [fleet, fboNetwork, pendingJobs] = await Promise.all([
        invokeDesktop<FleetSnapshotView | null>("onair_fleet_snapshot"),
        invokeDesktop<FboSnapshotView | null>("onair_fbo_snapshot"),
        invokeDesktop<JobSnapshotView | null>("onair_job_snapshot"),
      ]);
      if (fleet) {
        acceptFleetView(fleet);
        fleetLoadState = "ready";
      } else {
        fleetView = null;
      }
      if (fboNetwork) acceptFboView(fboNetwork);
      else fboView = null;
      jobView = pendingJobs;
      if (!fleet && !fboNetwork && !pendingJobs) fleetLoadState = "idle";
      await refreshTimeline();
      if (
        connection.connected &&
        (synchronizeAfterRestore || !fleet || !fboNetwork || !pendingJobs)
      ) {
        await synchronizeCompanyData("initial");
      }
    } catch (error) {
      fleetLoadState = "error";
      fleetError = safeError(error);
    }
  }

  function handleConnectionStatus(value: OnAirConnectionStatus): void {
    returnToPresent();
    connection = value;
    if (value.connected) {
      dialogNavigation = closedDialogNavigation<AppDialogSurface>();
      void restoreCompanySnapshots(true);
    } else {
      fleetError = "";
      void restoreCompanySnapshots(false);
    }
  }

  async function refreshPluginHost(): Promise<void> {
    if (!isDesktopRuntime()) return;
    try {
      pluginHost = await loadPluginHost();
    } catch (error) {
      pluginError = operationErrorMessage(
        error,
        "WyrmGrid could not read the local plugin workshop.",
      );
    }
  }

  async function refreshSecurityCentre(): Promise<void> {
    if (!isDesktopRuntime()) return;
    securityBusy = true;
    securityError = "";
    try {
      securityCentre = await loadSecurityCentre();
      securityCentreLoaded = true;
    } catch (error) {
      securityCentreLoaded = false;
      securityError = operationErrorMessage(
        error,
        "WyrmGrid could not read its local authorization record.",
      );
    } finally {
      securityBusy = false;
    }
  }

  async function refreshDataProtection(): Promise<void> {
    dataProtectionError = "";
    dataProtectionSuccess = "";
    if (!isDesktopRuntime()) {
      dataProtectionStatus = browserDataProtectionStatus;
      dataProtectionLoaded = true;
      return;
    }
    dataProtectionBusy = true;
    try {
      dataProtectionStatus = await loadDataProtectionStatus();
      dataProtectionLoaded = true;
    } catch (error) {
      dataProtectionLoaded = false;
      dataProtectionError = operationErrorMessage(
        error,
        "WyrmGrid could not read its encrypted-storage status.",
      );
    } finally {
      dataProtectionBusy = false;
    }
  }

  async function runPortableBackup(
    destination: string,
    password: string,
    passwordConfirmation: string,
  ): Promise<void> {
    dataProtectionBusy = true;
    dataProtectionError = "";
    dataProtectionSuccess = "";
    try {
      const backup = await createPortableBackup(
        destination,
        password,
        passwordConfirmation,
      );
      dataProtectionSuccess = translate("data-protection-backup-created", {
        time: new Date(backup.created_at).toLocaleString(),
      });
    } catch (error) {
      dataProtectionError = operationErrorMessage(
        error,
        "WyrmGrid could not create the encrypted portable backup.",
      );
    } finally {
      dataProtectionBusy = false;
    }
  }

  async function requestPortableBackupDestination(): Promise<string | null> {
    dataProtectionError = "";
    try {
      return await choosePortableBackupDestination();
    } catch (error) {
      dataProtectionError = operationErrorMessage(
        error,
        "WyrmGrid could not open the backup destination picker.",
      );
      return null;
    }
  }

  async function requestPortableBackupSource(): Promise<string | null> {
    dataProtectionError = "";
    try {
      return await choosePortableBackupSource();
    } catch (error) {
      dataProtectionError = operationErrorMessage(
        error,
        "WyrmGrid could not open the backup file picker.",
      );
      return null;
    }
  }

  async function runPortableRestore(
    source: string,
    password: string,
    replacementConfirmed: boolean,
  ): Promise<void> {
    dataProtectionBusy = true;
    dataProtectionError = "";
    dataProtectionSuccess = "";
    try {
      await preparePortableRestore(source, password, replacementConfirmed);
      dataProtectionStatus = {
        ...dataProtectionStatus,
        pending_restore: true,
      };
      dataProtectionSuccess = translate("data-protection-restore-prepared");
    } catch (error) {
      dataProtectionError = operationErrorMessage(
        error,
        "WyrmGrid could not prepare that encrypted portable backup.",
      );
    } finally {
      dataProtectionBusy = false;
    }
  }

  async function refreshDispatchStatus(): Promise<void> {
    if (!isDesktopRuntime()) return;
    try {
      dispatchStatus = await loadDispatchStatus();
    } catch (error) {
      dispatchError = operationErrorMessage(
        error,
        "WyrmGrid could not read the current Dispatch plan.",
      );
    }
  }

  async function importDispatchPlan(
    kind: SimBriefReferenceKind,
    reference: string,
    rememberReference: boolean,
  ): Promise<void> {
    if (dispatchBusy) return;
    dispatchBusy = true;
    dispatchError = "";
    try {
      dispatchStatus = isDesktopRuntime()
        ? await importLatestSimBriefPlan(kind, reference, rememberReference)
        : dispatchPreviewReady;
      simbriefAccountPreference = isDesktopRuntime()
        ? ((await loadSimBriefAccountPreference()) ?? undefined)
        : rememberReference
          ? { reference_kind: kind, reference }
          : undefined;
    } catch (error) {
      dispatchError = operationErrorMessage(
        error,
        "WyrmGrid could not import the latest SimBrief plan.",
      );
      await refreshDispatchStatus();
    } finally {
      dispatchBusy = false;
    }
  }

  async function clearCurrentDispatchPlan(): Promise<void> {
    if (dispatchBusy) return;
    dispatchBusy = true;
    dispatchError = "";
    try {
      dispatchStatus = isDesktopRuntime()
        ? await clearDispatchPlan()
        : dispatchPreviewEmpty;
    } catch (error) {
      dispatchError = operationErrorMessage(
        error,
        "WyrmGrid could not clear the session flight plan.",
      );
    } finally {
      dispatchBusy = false;
    }
  }

  async function openJobInDispatch(jobId: string): Promise<void> {
    if (dispatchBusy) return;
    dispatchBusy = true;
    dispatchError = "";
    try {
      if (isDesktopRuntime()) dispatchStatus = await selectDispatchJob(jobId);
      activeWorkspace = "dispatch";
    } catch (error) {
      dispatchError = operationErrorMessage(
        error,
        "WyrmGrid could not add that pending job to Dispatch.",
      );
    } finally {
      dispatchBusy = false;
    }
  }

  async function refreshCurrentDispatchWeather(): Promise<void> {
    if (dispatchBusy) return;
    dispatchBusy = true;
    dispatchError = "";
    try {
      dispatchStatus = isDesktopRuntime()
        ? await refreshDispatchWeather()
        : dispatchPreviewReady;
    } catch (error) {
      dispatchError = operationErrorMessage(
        error,
        "WyrmGrid could not refresh airport weather.",
      );
      await refreshDispatchStatus();
    } finally {
      dispatchBusy = false;
    }
  }

  function openForge(): void {
    pluginError = "";
    openRootDialog("forge");
    void refreshPluginHost();
  }

  async function runPluginAction(
    action: "approve" | "revoke" | "start" | "stop",
    pluginId: string,
    lifetime: AuthorizationGrantLifetime = "standing",
  ): Promise<void> {
    if (pluginBusy) return;
    pluginBusy = true;
    pluginError = "";
    try {
      if (!isDesktopRuntime()) {
        pluginHost =
          action === "approve"
            ? forgePreviewApproved
            : action === "start"
              ? forgePreviewRunning
              : action === "stop"
                ? forgePreviewApproved
                : forgePreviewStopped;
        securityCentre =
          action === "revoke" ? securityPreviewRevoked : securityPreviewGranted;
        securityCentreLoaded = true;
        return;
      }
      pluginHost =
        action === "approve"
          ? await approvePluginPermissions(pluginId, lifetime)
          : action === "revoke"
            ? await revokePluginPermissions(pluginId)
            : action === "start"
              ? await startPlugin(pluginId)
              : await stopPlugin(pluginId);
    } catch (error) {
      const message = operationErrorMessage(
        error,
        "WyrmGrid could not complete that plugin action.",
      );
      pluginError = message;
      if (showSecurityCentre) securityError = message;
      await refreshPluginHost();
    } finally {
      pluginBusy = false;
      if (showSecurityCentre && isDesktopRuntime())
        void refreshSecurityCentre();
    }
  }

  function initializeWorkspace(): void {
    if (workspaceInitialized) return;
    workspaceInitialized = true;

    if (!isDesktopRuntime()) {
      fleetView = atlasPreviewFleet;
      fboView = atlasPreviewFbos;
      timeline = hoardPreviewTimeline;
      timelineCursor = timeline.observation_times.length - 1;
      fleetLoadState = "ready";
      pluginHost = forgePreviewApproved;
      securityCentre = securityPreviewGranted;
      securityCentreLoaded = true;
      simulatorBridge = emptySimulatorBridge;
      dispatchStatus = dispatchPreviewReady;
      return;
    }

    void refreshPluginHost();
    void refreshSimulatorBridge();
    void refreshDispatchStatus();
    void loadSimBriefAccountPreference()
      .then(
        (preference) => (simbriefAccountPreference = preference ?? undefined),
      )
      .catch((error) => {
        dispatchError = operationErrorMessage(
          error,
          "WyrmGrid could not read the remembered SimBrief Pilot ID.",
        );
      });

    invokeDesktop<PlatformStatus>("platform_status")
      .then((value) => (status = value))
      .catch((error) => {
        fleetLoadState = "error";
        fleetError = operationErrorMessage(
          error,
          "WyrmGrid could not read its build status.",
        );
      });

    void initializeOnAirConnection();
  }

  async function initializeOnAirConnection(): Promise<void> {
    try {
      connection = await invokeDesktop<OnAirConnectionStatus>(
        "onair_connection_status",
      );
      if (!connection.connected) {
        try {
          const automatic = await autoConnectOnAirIfEnabled();
          if (automatic) connection = automatic.connection;
        } catch (error) {
          fleetError = operationErrorMessage(
            error,
            "The optional automatic OnAir connection was not completed. You can connect manually.",
          );
        }
      }
      await restoreCompanySnapshots(connection.connected);
    } catch (error) {
      fleetLoadState = "error";
      fleetError = operationErrorMessage(
        error,
        "WyrmGrid could not read connection state.",
      );
    }
  }

  async function initializeLegal(): Promise<void> {
    legalLoadState = "loading";
    legalError = "";
    try {
      legalStatus = await loadLegalStatus();
      legalTelemetryDraft = legalStatus.telemetry_enabled;
      await configureClientTelemetry(legalStatus.telemetry_enabled);
      const remainingDisplayTime = remainingLaunchDisplayTime(
        launchStartedAt,
        Date.now(),
      );
      if (remainingDisplayTime > 0) {
        await new Promise((resolve) =>
          window.setTimeout(resolve, remainingDisplayTime),
        );
      }
      legalLoadState = "ready";
      if (legalStatus.acknowledged) initializeWorkspace();
    } catch (error) {
      legalLoadState = "error";
      legalError = operationErrorMessage(
        error,
        "WyrmGrid could not read its local privacy preferences.",
      );
    }
  }

  async function initializeTheme(): Promise<void> {
    themeError = "";
    try {
      themeStatus = await loadThemeStatus();
      applyTheme(themeStatus.active_theme);
    } catch (error) {
      applyTheme(browserThemeStatus.active_theme);
      themeError = operationErrorMessage(
        error,
        "WyrmGrid could not read its local theme settings.",
      );
    }
  }

  async function initializeLanguage(): Promise<void> {
    languageError = "";
    try {
      languageStatus = await loadLanguageStatus();
      applyLanguage(languageStatus.active_pack);
    } catch (error) {
      applyLanguage(browserLanguageStatus.active_pack);
      languageError = operationErrorMessage(
        error,
        $translation("error-language-storage-unavailable"),
      );
    }
  }

  async function chooseLanguage(packId: string): Promise<void> {
    if (packId === languageStatus.selected_language_pack_id) return;
    languageBusy = true;
    languageError = "";
    try {
      languageStatus = await selectLanguagePack(packId);
      applyLanguage(languageStatus.active_pack);
    } catch (error) {
      languageError = operationErrorMessage(
        error,
        $translation("error-language-apply-failed"),
      );
    } finally {
      languageBusy = false;
    }
  }

  async function addLanguagePack(manifestJson: string): Promise<void> {
    languageBusy = true;
    languageError = "";
    try {
      languageStatus = await importLanguagePack(manifestJson);
      applyLanguage(languageStatus.active_pack);
    } catch (error) {
      languageError = operationErrorMessage(
        error,
        $translation("error-language-import-failed"),
      );
    } finally {
      languageBusy = false;
    }
  }

  async function chooseTheme(themeId: string): Promise<void> {
    if (themeId === themeStatus.selected_theme_id) return;
    themeBusy = true;
    themeError = "";
    try {
      themeStatus = await selectTheme(themeId);
      applyTheme(themeStatus.active_theme);
    } catch (error) {
      themeError = operationErrorMessage(
        error,
        "WyrmGrid could not apply that theme.",
      );
    } finally {
      themeBusy = false;
    }
  }

  async function addTheme(manifestJson: string): Promise<void> {
    themeBusy = true;
    themeError = "";
    try {
      themeStatus = await importTheme(manifestJson);
      applyTheme(themeStatus.active_theme);
    } catch (error) {
      themeError = operationErrorMessage(
        error,
        "WyrmGrid could not import that theme.",
      );
    } finally {
      themeBusy = false;
    }
  }

  async function initializeApplication(): Promise<void> {
    try {
      startupOptions = await loadStartupOptions();
    } catch {
      startupOptions = defaultStartupOptions;
    } finally {
      startupOptionsLoaded = true;
    }
    await initializeLanguage();
    await initializeTheme();
    await initializeDisplayPreferences();
    await initializeSimulatorPreferences();
    await initializeLegal();
  }

  async function initializeDisplayPreferences(): Promise<void> {
    try {
      displayPreferences = await loadDisplayPreferences();
    } catch (error) {
      settingsError = operationErrorMessage(
        error,
        "WyrmGrid could not read its local display settings.",
      );
    }
  }

  async function initializeSimulatorPreferences(): Promise<void> {
    if (!isDesktopRuntime()) return;
    try {
      [simulatorPreferences, simulatorBridge, simulatorRecording] =
        await Promise.all([
          loadSimulatorPreferences(),
          loadSimulatorBridge(),
          loadSimulatorRecording(),
        ]);
    } catch (error) {
      settingsError = operationErrorMessage(
        error,
        "WyrmGrid could not read its local simulator settings.",
      );
    }
  }

  async function saveSettings(
    preferences: DisplayPreferences,
    nextSimulatorPreferences: SimulatorPreferences,
    nextRecordingPreferences: SimulatorRecordingPreferences,
  ): Promise<void> {
    settingsBusy = true;
    settingsError = "";
    try {
      const [savedDisplay, savedSimulator, savedRecording] = await Promise.all([
        saveDisplayPreferences(preferences),
        saveSimulatorPreferences(nextSimulatorPreferences),
        saveSimulatorRecordingPreferences(nextRecordingPreferences),
      ]);
      displayPreferences = savedDisplay;
      simulatorPreferences = savedSimulator;
      simulatorRecording = {
        ...simulatorRecording,
        preferences: savedRecording,
      };
      dialogNavigation = closedDialogNavigation<AppDialogSurface>();
    } catch (error) {
      settingsError = operationErrorMessage(
        error,
        "WyrmGrid could not save its local display settings.",
      );
    } finally {
      settingsBusy = false;
    }
  }

  function openSettings(): void {
    settingsError = "";
    openRootDialog("settings");
    void refreshSimulatorBridge();
  }

  function openRootDialog(surface: AppDialogSurface): void {
    dialogNavigation = openDialogNavigation(surface);
  }

  function enterDialog(surface: AppDialogSurface): void {
    dialogNavigation = enterDialogSurface(dialogNavigation, surface);
  }

  function leaveDialog(): void {
    dialogNavigation = leaveDialogSurface(dialogNavigation);
  }

  function openSecurityCentre(): void {
    securityError = "";
    if (isDesktopRuntime()) {
      securityCentreLoaded = false;
      securityCentre = {
        ...securityPreviewEmpty,
        legal: legalStatus,
      };
    }
    void refreshSecurityCentre();
  }

  function openDataProtection(): void {
    dataProtectionError = "";
    dataProtectionSuccess = "";
    dataProtectionLoaded = !isDesktopRuntime();
    dataProtectionStatus = browserDataProtectionStatus;
    void refreshDataProtection();
  }

  function openLegalSettings(): void {
    legalTelemetryDraft = legalStatus.telemetry_enabled;
    legalError = "";
    showLegalDialog = true;
  }

  async function saveLegalChoice(): Promise<void> {
    legalBusy = true;
    legalError = "";
    try {
      legalStatus = legalStatus.acknowledged
        ? await updateTelemetryPreference(legalTelemetryDraft)
        : await acknowledgeLegal(legalTelemetryDraft);
      await configureClientTelemetry(legalStatus.telemetry_enabled);
      showLegalDialog = false;
      if (showSettingsPrivacy) leaveDialog();
      if (legalStatus.acknowledged) initializeWorkspace();
    } catch (error) {
      legalError = operationErrorMessage(
        error,
        "WyrmGrid could not save its local privacy preferences.",
      );
    } finally {
      legalBusy = false;
    }
  }

  function updateAutomaticSync(minutes: number): void {
    if (
      !AUTOMATIC_SYNC_OPTIONS.includes(
        minutes as (typeof AUTOMATIC_SYNC_OPTIONS)[number],
      )
    ) {
      return;
    }
    automaticSyncMinutes = minutes;
    localStorage.setItem(AUTOMATIC_SYNC_STORAGE_KEY, String(minutes));
  }

  $effect(() => {
    if (
      typeof window === "undefined" ||
      !connection.connected ||
      automaticSyncMinutes === 0
    ) {
      return;
    }

    const timer = window.setInterval(
      () => void synchronizeCompanyData("automatic"),
      automaticSyncMinutes * 60 * 1000,
    );
    return () => window.clearInterval(timer);
  });

  $effect(() => {
    if (
      typeof window === "undefined" ||
      !isDesktopRuntime() ||
      (!showSimulatorDialog && !simulatorProcessActive)
    ) {
      return;
    }
    const timer = window.setInterval(
      () =>
        void Promise.all([
          refreshSimulatorBridge(),
          refreshSimulatorRecording(),
        ]),
      1000,
    );
    return () => window.clearInterval(timer);
  });

  $effect(() => {
    if (
      typeof window === "undefined" ||
      !isDesktopRuntime() ||
      !pluginProcessActive
    ) {
      return;
    }
    const timer = window.setInterval(() => void refreshPluginHost(), 1000);
    return () => window.clearInterval(timer);
  });

  onMount(() => {
    const updateViewportMode = () => {
      viewportMode = viewportPresentation(
        window.innerWidth,
        window.innerHeight,
      );
    };
    updateViewportMode();
    window.addEventListener("resize", updateViewportMode);

    const savedAutomaticSync = Number.parseInt(
      localStorage.getItem(AUTOMATIC_SYNC_STORAGE_KEY) ?? "30",
      10,
    );
    if (
      AUTOMATIC_SYNC_OPTIONS.includes(
        savedAutomaticSync as (typeof AUTOMATIC_SYNC_OPTIONS)[number],
      )
    ) {
      automaticSyncMinutes = savedAutomaticSync;
    }

    void initializeApplication();
    return () => window.removeEventListener("resize", updateViewportMode);
  });
</script>

<svelte:head>
  <title>OnAir WyrmGrid</title>
</svelte:head>

{#if legalLoadState === "loading"}
  <LaunchScreen
    eyebrow={$translation("app-name")}
    message={$translation("app-preparing-privacy")}
    canvas={themeStatus.active_theme.colors.canvas}
    artworkEnabled={launchArtworkEnabled}
    lowResource={startupOptions.low_resource}
  />
{:else if legalLoadState === "error"}
  <LaunchScreen
    error
    eyebrow={$translation("app-settings-unavailable")}
    message={legalError}
    canvas={themeStatus.active_theme.colors.canvas}
    artworkEnabled={launchArtworkEnabled}
    lowResource={startupOptions.low_resource}
    retryLabel={$translation("action-try-again")}
    onretry={() => void initializeLegal()}
  />
{:else if legalStatus.acknowledged}
  <main
    class:compact-ui={startupOptions.compact_ui || viewportMode === "narrow"}
    class:short-ui={viewportMode === "short"}
    class:low-resource={startupOptions.low_resource}
    class="shell"
    inert={showLegalDialog ||
      showThemeDialog ||
      showLanguageDialog ||
      showDiagnosticsDialog ||
      showSettingsDialog ||
      showDataProtection ||
      showOpenSourceLicences ||
      showSecurityCentre ||
      showSimulatorDialog ||
      showTimelineDialog ||
      showForgeDialog}
  >
    <header class="topbar">
      <div class="brand-mark" aria-hidden="true">WG</div>
      <div class="brand-copy">
        <span class="eyebrow">OnAir</span>
        <h1>WyrmGrid</h1>
      </div>
      <nav aria-label={$translation("nav-primary")}>
        <button
          class="nav-item"
          class:active={activeWorkspace === "atlas"}
          type="button"
          onclick={() => (activeWorkspace = "atlas")}
          >{$translation("nav-atlas")}</button
        >
        <button class="nav-item" type="button" disabled
          >{$translation("nav-fleet")}</button
        >
        <button
          class="nav-item"
          class:active={activeWorkspace === "jobs"}
          type="button"
          onclick={() => (activeWorkspace = "jobs")}
          >{$translation("nav-jobs")}</button
        >
        <button
          class="nav-item"
          class:active={activeWorkspace === "dispatch"}
          type="button"
          onclick={() => {
            activeWorkspace = "dispatch";
            void refreshDispatchStatus();
          }}>{$translation("nav-dispatch")}</button
        >
        <button
          class="nav-item"
          class:active={showTimelineDialog}
          type="button"
          onclick={openHoardTimeline}>{$translation("nav-hoard")}</button
        >
        <button
          class="nav-item"
          class:active={showForgeDialog}
          type="button"
          onclick={openForge}>{$translation("nav-forge")}</button
        >
      </nav>
      <button
        class:connected={simulatorBridge.providers.some(
          (provider) =>
            provider.connection_state === "connected" &&
            !provider.telemetry_stale,
        )}
        class="simulator-pill"
        type="button"
        onclick={openSimulator}
      >
        {$translation("settings-simulator")}
      </button>
      <button class="diagnostics-pill" type="button" onclick={openDiagnostics}>
        Diagnostics
      </button>
      <button class="settings-pill" type="button" onclick={openSettings}>
        {$translation("settings-open")}
      </button>
      <button
        class:connected={connection.connected}
        class="connection-pill"
        type="button"
        onclick={() => openRootDialog("connection")}
      >
        <span></span>
        {connection.connected && connection.company
          ? connection.company.name
          : $translation("onair-connect")}
      </button>
    </header>

    {#if activeWorkspace === "atlas"}
      <section
        class:historical={timelineMode === "historical"}
        class="time-mode-bar"
      >
        <div class="time-mode-copy">
          <span class="time-mode-indicator" aria-hidden="true"></span>
          <strong
            >{timelineMode === "historical" ? "Historical" : "Live"}</strong
          >
          <span>
            {timelineMode === "historical"
              ? `${formatObservedAt(historicalData?.selected_at)} · retained Hoard view`
              : "Atlas follows the latest company observations"}
          </span>
        </div>
        <div class="time-mode-actions">
          {#if timelineMode === "historical"}
            <button type="button" onclick={returnToPresent}
              >Return to present</button
            >
          {/if}
          <button type="button" onclick={openHoardTimeline}
            >Open Hoard Timeline</button
          >
        </div>
      </section>

      <section class="workspace">
        <aside class="sidebar" aria-label="Map layers">
          <div class="section-heading">
            <div>
              <span class="eyebrow">WyrmGrid Atlas</span>
              <h2>Operations layers</h2>
            </div>
          </div>

          <div class="sync-controls">
            <button
              class="sync-button"
              class:refreshing={fleetLoadState === "loading"}
              type="button"
              disabled={!connection.connected}
              title="Synchronize current fleet and FBO observations with OnAir"
              onclick={() => void synchronizeCompanyData("manual")}
            >
              <span aria-hidden="true">↻</span>
              Synchronize OnAir
            </button>
            <label>
              <span>Automatic checks</span>
              <select
                value={automaticSyncMinutes}
                onchange={(event) =>
                  updateAutomaticSync(Number(event.currentTarget.value))}
              >
                <option value={0}>Off</option>
                <option value={15}>Every 15 minutes</option>
                <option value={30}>Every 30 minutes</option>
                <option value={60}>Every hour</option>
                <option value={120}>Every 2 hours</option>
              </select>
            </label>
            <small
              >Repeated requests are quietly rate-protected by WyrmGrid.</small
            >
          </div>

          <div class="layer-list">
            {#each layers as layer}
              <button
                class:muted={!layer.active}
                class="layer-row"
                aria-pressed={layer.active}
                disabled={!layer.available}
                title={layer.available
                  ? `Toggle ${layer.name}`
                  : `${layer.name} is planned for a later slice`}
                onclick={() => {
                  if (layer.id === "fleet") fleetVisible = !fleetVisible;
                  if (layer.id === "fbos") fboVisible = !fboVisible;
                  if (layer.id === "jobs") activeWorkspace = "jobs";
                  if (layer.id === "plugins")
                    pluginLayersVisible = !pluginLayersVisible;
                }}
              >
                <span class="layer-indicator"></span>
                <span>{layer.name}</span>
                <strong>{layer.count}</strong>
              </button>
            {/each}
          </div>

          {#if atlasFlightRoute}
            <div class="sidebar-note route-note">
              <span class="note-icon">↝</span>
              <p>
                <strong>Historical flight route</strong><br />
                {atlasFlightRoute.planned
                  ? `${atlasFlightRoute.planned.origin_icao} → ${atlasFlightRoute.planned.destination_icao}`
                  : "Recorded route without an associated plan"}
                · {atlasFlightRoute.recorded.represented_point_count.toLocaleString()}
                recorded points
              </p>
              <button
                type="button"
                onclick={() => (atlasFlightRoute = undefined)}>Clear</button
              >
            </div>
          {/if}

          <div
            class="sidebar-note"
            class:error-note={fleetLoadState === "error"}
          >
            <span class="note-icon"
              >{fleetLoadState === "error" ? "!" : "i"}</span
            >
            <p>
              {#if fleetLoadState === "loading"}
                Synchronizing company data with OnAir…
              {:else if fleetLoadState === "error"}
                {fleetError}
                {#if activeFleetView || activeFboView}
                  Previous Hoard observations remain visible where available.
                {/if}
              {:else if atlasCompany}
                {#if atlasAvailability === "offline"}
                  Offline Hoard snapshot for {atlasCompany.name}.
                {:else if atlasAvailability === "cached"}
                  Cached Hoard snapshot for {atlasCompany.name}; synchronization
                  is pending.
                {:else if atlasAvailability === "preview"}
                  Synthetic browser-preview company data.
                {:else}
                  Live company data for {atlasCompany.name}.
                {/if}
                {countedLabel(aircraft.length, "aircraft", "aircraft")} and
                {countedLabel(fbos.length, "FBO")} received;
                {plottedAircraftCount + plottedFboCount} Atlas points mappable.
                {formatObservedAt(atlasObservedAt)}.
              {:else if connection.connected}
                OnAir is connected. Synchronize company data to populate Atlas.
              {:else}
                Connect an OnAir company to begin. Credentials remain only in
                memory for this session.
              {/if}
            </p>
          </div>
        </aside>

        <section class="map-stage" aria-label="Universal operations map">
          <AtlasMap
            {aircraft}
            {fbos}
            {fleetVisible}
            {fboVisible}
            pluginLayers={pluginHost.layers}
            {pluginLayersVisible}
            flightRoute={atlasFlightRoute}
            {selectedAircraftId}
            {selectedFboId}
            onselectaircraft={(aircraftId) => {
              selectedAircraftId = aircraftId;
              selectedFboId = null;
            }}
            onselectfbo={(fboId) => {
              selectedFboId = fboId;
              selectedAircraftId = null;
            }}
          />
          <div class="map-wash"></div>
          <div class="map-title">
            <span class="eyebrow">Universal operations map</span>
            <strong>See the network. Command the skies.</strong>
            {#if atlasAvailability && atlasAvailability !== "live"}
              <span
                class:offline={atlasAvailability === "offline"}
                class="data-mode-badge"
              >
                {fleetAvailabilityLabel} · {fleetStorageLabel}
              </span>
            {/if}
          </div>
          <div class="readiness-card">
            <span class="eyebrow">Atlas readiness</span>
            <div class="readiness-value">
              {activeFleetView || activeFboView
                ? `${countedLabel(plottedAircraftCount, "aircraft", "aircraft")} · ${countedLabel(plottedFboCount, "FBO")} mapped`
                : "Awaiting company data"}
            </div>
            <dl>
              <div>
                <dt>Source</dt>
                <dd>{atlasCompany ? fleetSourceLabel : "Not connected"}</dd>
              </div>
              <div>
                <dt>State</dt>
                <dd>{fleetAvailabilityLabel}</dd>
              </div>
              <div>
                <dt>Plugin API</dt>
                <dd>v{status.plugin_api_version}</dd>
              </div>
              <div>
                <dt>Build</dt>
                <dd>{status.version}</dd>
              </div>
            </dl>
            {#if !activeFleetView}<WyrmChart spec={foundationChart} />{/if}
          </div>
        </section>

        <aside class="inspector" aria-label="Selection inspector">
          <span class="eyebrow">Inspector</span>
          {#if selectedAircraft}
            <h2>{displayRegistration(selectedAircraft)}</h2>
            <p>{selectedAircraft.model ?? "Aircraft type unavailable"}</p>

            <div class="selection-details">
              <article>
                <span>Current airport</span>
                <strong
                  >{selectedAircraft.current_airport?.icao ||
                    "Not reported"}</strong
                >
                {#if selectedAircraft.current_airport?.name}
                  <small>{selectedAircraft.current_airport.name}</small>
                {/if}
              </article>
              <article>
                <span>Coordinates</span>
                <strong>{formatCoordinates(selectedAircraft)}</strong>
              </article>
              <article>
                <span>Provenance</span>
                <strong>{fleetSourceLabel}</strong>
                <small
                  >{formatObservedAt(
                    fleetSnapshot?.provenance.observed_at,
                  )}</small
                >
              </article>
            </div>
          {:else if selectedFbo}
            <h2>{selectedFbo.name ?? "Unnamed FBO"}</h2>
            <p>Company FBO network location</p>

            <div class="selection-details">
              <article>
                <span>Airport</span>
                <strong>{selectedFbo.airport?.icao || "Not reported"}</strong>
                {#if selectedFbo.airport?.name}<small
                    >{selectedFbo.airport.name}</small
                  >{/if}
              </article>
              <article>
                <span>Coordinates</span>
                <strong>{formatFboCoordinates(selectedFbo)}</strong>
              </article>
              <article>
                <span>Provenance</span>
                <strong>{fboSourceLabel}</strong>
                <small
                  >{formatObservedAt(
                    activeFboView?.snapshot.provenance.observed_at,
                  )}</small
                >
              </article>
            </div>
          {:else if atlasFlightRoute}
            <h2>
              {atlasFlightRoute.planned
                ? `${atlasFlightRoute.planned.origin_icao} → ${atlasFlightRoute.planned.destination_icao}`
                : "Recorded flight"}
            </h2>
            <p>
              Planned and recorded paths remain separate, including every known
              gap.
            </p>

            <div class="selection-details">
              <article>
                <span>Recorded path</span>
                <strong
                  >{atlasFlightRoute.recorded.represented_point_count.toLocaleString()}
                  points</strong
                >
                <small
                  >{atlasFlightRoute.recorded.method.replaceAll(
                    "_",
                    " ",
                  )}</small
                >
              </article>
              <article>
                <span>Planned path</span>
                <strong
                  >{atlasFlightRoute.planned?.points.length.toLocaleString() ??
                    "Unavailable"}</strong
                >
                <small
                  >Attributed to {atlasFlightRoute.planned?.provider ??
                    "no plan provider"}</small
                >
              </article>
              <article>
                <span>Unresolved plan legs</span>
                <strong
                  >{atlasFlightRoute.planned?.unresolved_legs.length.toLocaleString() ??
                    "0"}</strong
                >
                <small>Unresolved coordinates are not plotted.</small>
              </article>
            </div>
          {:else}
            <h2>Nothing selected</h2>
            <p>
              Select a mapped aircraft or FBO to inspect its current operational
              context.
            </p>

            <div class="empty-radar" aria-hidden="true">
              <span></span><span></span><span></span>
              <i></i>
            </div>
          {/if}

          <div class="status-grid">
            <article>
              <span>OnAir</span><strong
                >{connection.connected ? "Connected" : "Not connected"}</strong
              >
            </article>
            <article>
              <span>Fleet</span><strong>{fleetResourceAvailabilityLabel}</strong
              >
            </article>
            <article>
              <span>FBOs</span><strong>{fboAvailabilityLabel}</strong>
            </article>
            <article>
              <span>Storage</span><strong>{fleetStorageLabel}</strong>
            </article>
          </div>
        </aside>
      </section>
    {:else if activeWorkspace === "jobs"}
      <section class="time-mode-bar dispatch-mode-bar">
        <div class="time-mode-copy">
          <span class="time-mode-indicator" aria-hidden="true"></span>
          <strong>{$translation("jobs-read-only")}</strong>
          <span>{$translation("jobs-read-only-banner")}</span>
        </div>
        <div class="time-mode-actions">
          <span>{$translation("jobs-contract-version")}</span>
        </div>
      </section>

      <JobsWorkspace
        view={jobView}
        busy={fleetLoadState === "loading"}
        errorMessage={fleetError}
        onsynchronize={() => void synchronizeCompanyData("manual")}
        ondispatch={(jobId) => void openJobInDispatch(jobId)}
      />
    {:else}
      <section class="time-mode-bar dispatch-mode-bar">
        <div class="time-mode-copy">
          <span class="time-mode-indicator" aria-hidden="true"></span>
          <strong>Read only</strong>
          <span>
            SimBrief plans remain external calculations and live only in this
            WyrmGrid session
          </span>
        </div>
        <div class="time-mode-actions">
          <span>Snapshot contract · v1</span>
        </div>
      </section>

      <DispatchWorkspace
        status={dispatchStatus}
        accountPreference={simbriefAccountPreference}
        busy={dispatchBusy}
        errorMessage={dispatchError}
        onimport={(kind, reference, rememberReference) =>
          void importDispatchPlan(kind, reference, rememberReference)}
        onweather={() => void refreshCurrentDispatchWeather()}
        onclear={() => void clearCurrentDispatchPlan()}
      />
    {/if}

    <footer>
      <span>{status.application} · {status.mode}</span>
      {#if startupOptions.low_resource}
        <span>Low-resource presentation</span>
      {:else if startupOptions.compact_ui}
        <span>Compact presentation</span>
      {/if}
      <span>{$translation("footer-unaffiliated")}</span>
    </footer>
  </main>

  <ConnectionDialog
    open={showConnectionDialog}
    status={connection}
    onclose={leaveDialog}
    onstatuschange={handleConnectionStatus}
  />

  <DiagnosticsDialog
    open={showDiagnosticsDialog}
    log={diagnosticLog}
    busy={diagnosticsBusy}
    errorMessage={diagnosticsError}
    onrefresh={() => void refreshDiagnostics()}
    onclear={() => void clearDiagnostics()}
    onclose={leaveDialog}
  />

  <SimulatorDialog
    open={showSimulatorDialog}
    status={simulatorBridge}
    busy={simulatorBusy}
    errorMessage={simulatorError}
    {displayPreferences}
    recordingStatus={simulatorRecording}
    recordingSession={simulatorRecordingSession}
    recordingDebrief={simulatorRecordingDebrief}
    recordingBusy={simulatorRecordingBusy}
    onrefresh={() => void refreshSimulatorBridge()}
    onstart={(providerId) => void runSimulatorAction("start", providerId)}
    onstop={(providerId) => void runSimulatorAction("stop", providerId)}
    onrecordstart={() => void runRecordingAction("start")}
    onrecordstop={() => void runRecordingAction("stop")}
    onsessionselect={(sessionId) => void selectRecordingSession(sessionId)}
    onsessiondelete={(sessionId) =>
      void runRecordingAction("delete", sessionId)}
    ondeleteall={() => void runRecordingAction("delete_all")}
    onpin={(sessionId, pinned) => void setRecordingPinned(sessionId, pinned)}
    onpage={(sessionId, sampleOffset) =>
      void pageRecordingSession(sessionId, sampleOffset)}
    onexport={(sessionId, format) => void exportRecording(sessionId, format)}
    onviewatlas={openRecordingRouteInAtlas}
    onclose={leaveDialog}
  />

  <SettingsDialog
    open={showSettingsDialog}
    preferences={displayPreferences}
    {simulatorPreferences}
    recordingPreferences={simulatorRecording.preferences}
    simulatorProviders={simulatorBridge.providers}
    busy={settingsBusy}
    errorMessage={settingsError}
    onsave={(preferences, nextSimulatorPreferences, nextRecordingPreferences) =>
      void saveSettings(
        preferences,
        nextSimulatorPreferences,
        nextRecordingPreferences,
      )}
    onappearance={() => {
      themeError = "";
      enterDialog("theme");
    }}
    onlanguage={() => {
      languageError = "";
      enterDialog("language");
    }}
    onprivacy={() => {
      enterDialog("privacy");
      openLegalSettings();
    }}
    onsecurity={() => {
      enterDialog("security");
      openSecurityCentre();
    }}
    ondataprotection={() => {
      enterDialog("data_protection");
      openDataProtection();
    }}
    onlicenses={() => {
      enterDialog("licenses");
    }}
    onclose={leaveDialog}
  />

  <DataProtectionDialog
    open={showDataProtection}
    desktopRuntime={isDesktopRuntime()}
    loaded={dataProtectionLoaded}
    status={dataProtectionStatus}
    busy={dataProtectionBusy}
    errorMessage={dataProtectionError}
    successMessage={dataProtectionSuccess}
    onrefresh={() => void refreshDataProtection()}
    onchoosebackup={requestPortableBackupDestination}
    onchooserestore={requestPortableBackupSource}
    onbackup={(destination, password, confirmation) =>
      void runPortableBackup(destination, password, confirmation)}
    onrestore={(source, password, confirmed) =>
      void runPortableRestore(source, password, confirmed)}
    onlicenses={() => {
      enterDialog("licenses");
    }}
    onclose={leaveDialog}
  />

  <OpenSourceLicencesDialog
    open={showOpenSourceLicences}
    onclose={leaveDialog}
  />

  <SecurityCentreDialog
    open={showSecurityCentre}
    status={securityCentre}
    loaded={securityCentreLoaded}
    busy={securityBusy || pluginBusy}
    errorMessage={securityError}
    onrefresh={() => void refreshSecurityCentre()}
    onrevoke={(subjectId) => void runPluginAction("revoke", subjectId)}
    onprivacy={() => {
      enterDialog("privacy");
      openLegalSettings();
    }}
    onclose={leaveDialog}
  />

  <ThemeDialog
    open={showThemeDialog}
    status={themeStatus}
    desktopRuntime={isDesktopRuntime()}
    busy={themeBusy}
    errorMessage={themeError}
    onselect={(themeId) => void chooseTheme(themeId)}
    onimport={(manifestJson) => void addTheme(manifestJson)}
    onclose={leaveDialog}
  />

  <LanguageDialog
    open={showLanguageDialog}
    status={languageStatus}
    desktopRuntime={isDesktopRuntime()}
    busy={languageBusy}
    errorMessage={languageError}
    onselect={(packId) => void chooseLanguage(packId)}
    onimport={(manifestJson) => void addLanguagePack(manifestJson)}
    onclose={leaveDialog}
  />

  <HoardTimelineDialog
    open={showTimelineDialog}
    mode={timelineMode}
    {timeline}
    cursor={timelineCursor}
    growthChart={timelineGrowthChart}
    fboGrowthChart={timelineFboGrowthChart}
    compositionChart={timelineFleetCompositionChart}
    {displayPreferences}
    recordingStatus={simulatorRecording}
    recordingSession={simulatorRecordingSession}
    recordingDebrief={simulatorRecordingDebrief}
    recordingBusy={simulatorRecordingBusy}
    recordingError={simulatorError}
    busy={timelineBusy}
    errorMessage={timelineError}
    oncursorchange={(cursor) => (timelineCursor = cursor)}
    onview={() => void viewHistoricalMoment()}
    onreturn={returnToPresent}
    onrecordingselect={(sessionId) => void selectRecordingSession(sessionId)}
    onrecordingdelete={(sessionId) =>
      void runRecordingAction("delete", sessionId)}
    onrecordingdeleteall={() => void runRecordingAction("delete_all")}
    onrecordingpin={(sessionId, pinned) =>
      void setRecordingPinned(sessionId, pinned)}
    onrecordingpage={(sessionId, sampleOffset) =>
      void pageRecordingSession(sessionId, sampleOffset)}
    onrecordingexport={(sessionId, format) =>
      void exportRecording(sessionId, format)}
    onrecordingviewatlas={openRecordingRouteInAtlas}
    onclose={leaveDialog}
  />

  <ForgeDialog
    open={showForgeDialog}
    status={pluginHost}
    busy={pluginBusy}
    errorMessage={pluginError}
    onapprove={(pluginId, lifetime) =>
      void runPluginAction("approve", pluginId, lifetime)}
    onrevoke={(pluginId) => void runPluginAction("revoke", pluginId)}
    onstart={(pluginId) => void runPluginAction("start", pluginId)}
    onstop={(pluginId) => void runPluginAction("stop", pluginId)}
    onclose={leaveDialog}
  />
{/if}

<LegalDialog
  open={legalLoadState === "ready" &&
    (!legalStatus.acknowledged || showLegalDialog || showSettingsPrivacy)}
  required={!legalStatus.acknowledged}
  status={legalStatus}
  telemetryEnabled={legalTelemetryDraft}
  busy={legalBusy}
  errorMessage={legalError}
  ontelemetrychange={(enabled) => (legalTelemetryDraft = enabled)}
  onsubmit={() => void saveLegalChoice()}
  onclose={() => {
    showLegalDialog = false;
    if (showSettingsPrivacy) leaveDialog();
  }}
/>
