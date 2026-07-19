<script lang="ts">
  import type {
    ExpressionSpecification,
    GeoJSONSource,
    ImageSource,
    Map as MapLibreMap,
  } from "maplibre-gl";
  import "maplibre-gl/dist/maplibre-gl.css";
  import { onMount } from "svelte";
  import { translation } from "$lib/i18n/runtime";
  import type {
    AtlasRouteView,
    RouteWeatherAnalysis,
  } from "$lib/dispatch/types";
  import type {
    PublishedPluginLayer,
    PublishedPluginWeatherLayer,
  } from "$lib/forge/types";
  import { activeTheme } from "$lib/theme/runtime";
  import {
    weatherFitCoordinates,
    weatherMapSignature,
    weatherPointCoordinates,
    weatherStationFeatures,
    weatherWindFeatures,
  } from "$lib/weather/atlasWeather";
  import type { FlightWeatherMapView } from "$lib/weather/types";
  import type { WeatherGraphicsPreferences } from "$lib/settings/types";
  import {
    lightningFlashOpacity,
    resolveWeatherGraphicsPolicy,
  } from "$lib/weather/graphics";
  import type {
    WeatherRenderer,
    WeatherRendererBackend,
    WeatherRendererStatus,
  } from "$lib/weather/renderer/types";
  import type { AdaptiveWeatherQuality } from "$lib/weather/renderer/quality";
  import {
    weatherProjectionSurfaceVisibility,
    weatherScreenSurfaceVisibility,
  } from "$lib/weather/renderer/projectionVisibility";
  import { buildWeatherRenderScene } from "$lib/weather/renderer/weatherRenderScene";
  import { presentWeatherRendererStatus } from "$lib/weather/renderer/statusPresentation";
  import {
    longestRadarTimeline,
    pluginRadarTimelines,
    pluginWeatherGridFeatures,
    selectedRadarFrames,
  } from "$lib/weather/pluginWeather";
  import { routeWeatherLineFeatures } from "$lib/weather/routeWeather";
  import {
    pluginRadarCoverageFeatures,
    pluginWeatherGridCoverageFeatures,
  } from "$lib/weather/weatherCoverage";
  import {
    WEATHER_ZONE_COLORS,
    weatherZonePatternExpression,
    weatherZonePatternId,
    weatherZonePatternImages,
  } from "$lib/weather/weatherCoveragePatterns";
  import { ATLAS_HOME_CENTER, balancedOverviewCoordinates } from "./camera";
  import type { AtlasView } from "./preferences";
  import { daylightFeatureCollection } from "./daylight";
  import {
    ADMINISTRATIVE_REGION_LABEL_BANDS,
    administrativeRegionFromMapFeature,
    ATLAS_ADMIN1_DATASET_URL,
  } from "./regions";
  import {
    flightRouteSignature,
    routeFitCoordinates,
    routeLineFeatures,
    routeMarkerFeatures,
    routePointCoordinates,
  } from "./flightRoute";
  import type {
    AircraftSummary,
    AtlasAdministrativeRegion,
    AtlasFlightRoute,
    AtlasFocusRequest,
    FboSummary,
  } from "./types";
  import {
    atlasRouteBounds,
    atlasRouteGeoJson,
    findRouteFeature,
  } from "./route";

  let {
    aircraft,
    fbos,
    fleetVisible,
    fboVisible,
    pluginLayers,
    pluginLayersVisible,
    pluginWeatherLayers,
    pluginWeatherVisible,
    flightRoute,
    weather,
    weatherVisible,
    daylightVisible,
    daylightAt,
    weatherCoverageVisible,
    weatherGraphics,
    regionsVisible,
    lowResource,
    selectedRegionId,
    selectedRoutePointId,
    selectedWeatherStationId,
    route,
    routeWeather,
    routeVisible,
    selectedAircraftId,
    selectedFboId,
    selectedRouteFeatureId,
    focusRequest,
    initialView,
    onselectaircraft,
    onselectfbo,
    onselectroutepoint,
    onselectweatherstation,
    onselectregion,
    onhoverregion,
    onselectroutefeature,
    onviewchange,
  }: {
    aircraft: AircraftSummary[];
    fbos: FboSummary[];
    fleetVisible: boolean;
    fboVisible: boolean;
    pluginLayers: PublishedPluginLayer[];
    pluginLayersVisible: boolean;
    pluginWeatherLayers: PublishedPluginWeatherLayer[];
    pluginWeatherVisible: boolean;
    flightRoute?: AtlasFlightRoute;
    weather?: FlightWeatherMapView;
    weatherVisible: boolean;
    daylightVisible: boolean;
    daylightAt?: string;
    weatherCoverageVisible: boolean;
    weatherGraphics: WeatherGraphicsPreferences;
    regionsVisible: boolean;
    lowResource: boolean;
    selectedRegionId?: string;
    selectedRoutePointId?: string;
    selectedWeatherStationId?: string;
    route?: AtlasRouteView;
    routeWeather?: RouteWeatherAnalysis;
    routeVisible: boolean;
    selectedAircraftId: string | null;
    selectedFboId: string | null;
    selectedRouteFeatureId: string | null;
    focusRequest: AtlasFocusRequest | null;
    initialView?: AtlasView;
    onselectaircraft: (aircraftId: string) => void;
    onselectfbo: (fboId: string) => void;
    onselectroutepoint: (pointId: string) => void;
    onselectweatherstation: (stationId: string) => void;
    onselectregion: (region: AtlasAdministrativeRegion) => void;
    onhoverregion: (region?: AtlasAdministrativeRegion) => void;
    onselectroutefeature: (featureId: string) => void;
    onviewchange: (view: AtlasView) => void;
  } = $props();

  const REGION_SOURCE_ID = "wyrmgrid-administrative-regions";
  const REGION_FILL_LAYER_ID = "wyrmgrid-administrative-region-fills";
  const REGION_BOUNDARY_LAYER_ID = "wyrmgrid-administrative-region-boundaries";
  const REGION_HALO_LAYER_ID = "wyrmgrid-administrative-region-halo";
  const REGION_LABEL_LAYER_PREFIX = "wyrmgrid-administrative-region-labels";
  const REGION_LABEL_LAYER_IDS = ADMINISTRATIVE_REGION_LABEL_BANDS.map(
    (band) => `${REGION_LABEL_LAYER_PREFIX}-${band.id}`,
  );

  const FLEET_SOURCE_ID = "wyrmgrid-fleet";
  const FLEET_LAYER_ID = "wyrmgrid-fleet-aircraft";
  const FLEET_LABEL_LAYER_ID = "wyrmgrid-fleet-labels";
  const FBO_SOURCE_ID = "wyrmgrid-fbos";
  const FBO_LAYER_ID = "wyrmgrid-fbo-network";
  const FBO_LABEL_LAYER_ID = "wyrmgrid-fbo-labels";
  const PLUGIN_SOURCE_ID = "wyrmgrid-plugin-layers";
  const PLUGIN_LAYER_ID = "wyrmgrid-plugin-points";
  const PLUGIN_LABEL_LAYER_ID = "wyrmgrid-plugin-labels";
  const PLUGIN_WEATHER_GRID_SOURCE_ID = "wyrmgrid-plugin-weather-grid";
  const PLUGIN_WEATHER_COVERAGE_SOURCE_ID = "wyrmgrid-plugin-weather-coverage";
  const PLUGIN_WEATHER_COVERAGE_FILL_LAYER_ID =
    "wyrmgrid-plugin-weather-coverage-fill";
  const PLUGIN_WEATHER_COVERAGE_PATTERN_LAYER_ID =
    "wyrmgrid-plugin-weather-coverage-pattern";
  const PLUGIN_WEATHER_COVERAGE_LINE_LAYER_ID =
    "wyrmgrid-plugin-weather-coverage-line";
  const PLUGIN_RADAR_COVERAGE_SOURCE_ID = "wyrmgrid-plugin-radar-coverage";
  const PLUGIN_RADAR_COVERAGE_FILL_LAYER_ID =
    "wyrmgrid-plugin-radar-coverage-fill";
  const PLUGIN_RADAR_COVERAGE_PATTERN_LAYER_ID =
    "wyrmgrid-plugin-radar-coverage-pattern";
  const PLUGIN_RADAR_COVERAGE_LINE_LAYER_ID =
    "wyrmgrid-plugin-radar-coverage-line";
  const PLUGIN_WEATHER_ATMOSPHERE_LAYER_ID =
    "wyrmgrid-plugin-weather-atmosphere";
  const PLUGIN_WEATHER_CLOUD_HIGHLIGHT_LAYER_ID =
    "wyrmgrid-plugin-weather-cloud-highlight";
  const PLUGIN_WEATHER_PRECIPITATION_LAYER_ID =
    "wyrmgrid-plugin-weather-precipitation";
  const PLUGIN_WEATHER_LIGHTNING_FLASH_LAYER_ID =
    "wyrmgrid-plugin-weather-lightning-flash";
  const PLUGIN_WEATHER_LIGHTNING_LAYER_ID = "wyrmgrid-plugin-weather-lightning";
  const PLUGIN_WEATHER_GRID_LAYER_ID = "wyrmgrid-plugin-weather-grid-points";
  const PLUGIN_RADAR_PREFIX = "wyrmgrid-plugin-radar";
  const DAYLIGHT_SOURCE_ID = "wyrmgrid-daylight";
  const DAYLIGHT_SHADE_LAYER_ID = "wyrmgrid-daylight-shade";
  const DAYLIGHT_TERMINATOR_LAYER_ID = "wyrmgrid-daylight-terminator";
  const ROUTE_SOURCE_ID = "wyrmgrid-flight-routes";
  const ROUTE_MARKER_SOURCE_ID = "wyrmgrid-flight-route-markers";
  const PLANNED_ROUTE_LAYER_ID = "wyrmgrid-planned-flight-route";
  const RECORDED_ROUTE_LAYER_ID = "wyrmgrid-recorded-flight-route";
  const ROUTE_MARKER_LAYER_ID = "wyrmgrid-flight-route-markers";
  const ROUTE_LABEL_LAYER_ID = "wyrmgrid-flight-route-labels";
  const WEATHER_SOURCE_ID = "wyrmgrid-flight-weather";
  const WEATHER_WIND_SOURCE_ID = "wyrmgrid-flight-weather-wind";
  const WEATHER_ATMOSPHERE_LAYER_ID = "wyrmgrid-flight-weather-atmosphere";
  const WEATHER_COVERAGE_OUTER_LAYER_ID =
    "wyrmgrid-flight-weather-coverage-outer";
  const WEATHER_COVERAGE_INNER_LAYER_ID =
    "wyrmgrid-flight-weather-coverage-inner";
  const WEATHER_COVERAGE_PATTERN_LAYER_ID =
    "wyrmgrid-flight-weather-coverage-pattern";
  const WEATHER_CLOUD_SHADOW_LAYER_ID = "wyrmgrid-flight-weather-cloud-shadow";
  const WEATHER_CLOUD_BODY_LAYER_ID = "wyrmgrid-flight-weather-cloud-body";
  const WEATHER_CLOUD_HIGHLIGHT_LAYER_ID =
    "wyrmgrid-flight-weather-cloud-highlight";
  const WEATHER_EFFECT_LAYER_ID = "wyrmgrid-flight-weather-effects";
  const WEATHER_PRECIPITATION_LAYER_ID =
    "wyrmgrid-flight-weather-precipitation";
  const WEATHER_LIGHTNING_FLASH_LAYER_ID =
    "wyrmgrid-flight-weather-lightning-flash";
  const WEATHER_LIGHTNING_LAYER_ID = "wyrmgrid-flight-weather-lightning";
  const WEATHER_DUST_LAYER_ID = "wyrmgrid-flight-weather-dust";
  const WEATHER_DUST_CORE_LAYER_ID = "wyrmgrid-flight-weather-dust-core";
  const WEATHER_WIND_LAYER_ID = "wyrmgrid-flight-weather-wind-paths";
  const WEATHER_WIND_TIP_LAYER_ID = "wyrmgrid-flight-weather-wind-tips";
  const WEATHER_LAYER_ID = "wyrmgrid-flight-weather-stations";
  const WEATHER_LABEL_LAYER_ID = "wyrmgrid-flight-weather-labels";
  const DISPATCH_ROUTE_SOURCE_ID = "wyrmgrid-dispatch-route";
  const ROUTE_WEATHER_SOURCE_ID = "wyrmgrid-route-weather";
  const ROUTE_WEATHER_HALO_LAYER_ID = "wyrmgrid-route-weather-halo";
  const ROUTE_WEATHER_SUPPORTED_LAYER_ID = "wyrmgrid-route-weather-supported";
  const ROUTE_WEATHER_CURRENT_CONTEXT_LAYER_ID =
    "wyrmgrid-route-weather-current-context";
  const ROUTE_WEATHER_UNAVAILABLE_LAYER_ID =
    "wyrmgrid-route-weather-unavailable";
  const DISPATCH_ROUTE_LINE_LAYER_ID = "wyrmgrid-dispatch-route-line";
  const DISPATCH_ROUTE_POINT_LAYER_ID = "wyrmgrid-dispatch-route-points";
  const DISPATCH_ROUTE_LABEL_LAYER_ID = "wyrmgrid-dispatch-route-labels";
  const DUST_OUTER_COLOR = "#9f764b";
  const DUST_CORE_COLOR = "#d2ad72";
  const LIGHTNING_COLOR = "#ffe79a";
  const DAYLIGHT_TERMINATOR_COLOR = "#e3ad62";

  let mapContainer: HTMLDivElement;
  let weatherCanvas: HTMLCanvasElement;
  let map: MapLibreMap | undefined;
  let mapReady = $state(false);
  let fittedAtlasSignature = "";
  let hoveredRegionFeatureId: string | number | undefined;
  let selectedRegionFeatureId: string | number | undefined;
  let prefersReducedMotion = $state(false);
  let weatherAnimationFrame: number | undefined;
  let weatherAnimationTime = 0;
  let pluginRadarLayerIds = new Set<string>();
  let pluginRadarFrameVersions = new Map<string, string>();
  let radarFrameIndex = $state(0);
  let radarPlaying = $state(true);
  let radarTimelineSignature = "";
  let daylightSourceSignature = "";
  let weatherRenderer: WeatherRenderer | undefined;
  let weatherRendererGeneration = 0;
  let weatherRendererInitializationKey: string | undefined;
  let weatherRendererFailureKey: string | undefined;
  let weatherRendererStatus = $state<WeatherRendererStatus>({
    state: "disabled",
  });
  const weatherPolicy = $derived(
    resolveWeatherGraphicsPolicy(
      weatherGraphics,
      lowResource,
      prefersReducedMotion,
    ),
  );
  const radarTimelines = $derived(pluginRadarTimelines(pluginWeatherLayers));
  const radarFrameCount = $derived(
    Math.max(0, ...radarTimelines.map((timeline) => timeline.frames.length)),
  );
  const radarStaticPresentation = $derived(lowResource || prefersReducedMotion);
  const activeRadarFrames = $derived(
    selectedRadarFrames(
      radarTimelines,
      radarFrameIndex,
      radarStaticPresentation,
    ),
  );
  const primaryRadarTimeline = $derived(longestRadarTimeline(radarTimelines));
  const primaryRadarFrame = $derived(
    primaryRadarTimeline
      ? selectedRadarFrames(
          [primaryRadarTimeline],
          radarFrameIndex,
          radarStaticPresentation,
        )[0]
      : undefined,
  );
  const primaryRadarHasCoverageMask = $derived(
    primaryRadarFrame?.tiles.some((tile) => tile.coverage_url) ?? false,
  );
  const primaryRadarFramePosition = $derived(
    primaryRadarFrame && primaryRadarTimeline
      ? primaryRadarTimeline.frames.findIndex(
          (frame) => frame.id === primaryRadarFrame.id,
        ) + 1
      : 0,
  );
  const weatherRenderScene = $derived(
    buildWeatherRenderScene(weather, pluginWeatherLayers),
  );
  const visibleWeatherRenderCells = $derived(
    weatherRenderScene.cells.filter((cell) =>
      cell.source === "airport" ? weatherVisible : pluginWeatherVisible,
    ),
  );
  const visibleWeatherRenderScene = $derived({
    signature: `${weatherVisible}:${pluginWeatherVisible}:${weatherRenderScene.signature}`,
    cells: visibleWeatherRenderCells,
  });
  const threeWeatherActive = $derived(
    weatherRendererStatus.state === "ready" && weatherPolicy.atmosphere,
  );
  const mapWeatherEffectsActive = $derived(!threeWeatherActive);
  const plottedWeatherStationCount = $derived(
    weatherStationFeatures(weather).features.length,
  );
  const plottedWeatherWindCount = $derived(
    weatherWindFeatures(weather).features.filter(
      ({ properties }) => properties.feature_type === "wind_path",
    ).length,
  );
  const activeWeatherEffectCount = $derived(
    weatherStationFeatures(weather).features.filter(
      ({ properties }) => properties.effect !== "none",
    ).length,
  );
  const weatherStatusPresentation = $derived(
    presentWeatherRendererStatus(
      {
        profile: weatherPolicy.profile,
        rendererStatus: weatherRendererStatus,
        lowResource,
        reducedMotion: prefersReducedMotion,
        stationCount: plottedWeatherStationCount,
        windCount: plottedWeatherWindCount,
        effectCount: activeWeatherEffectCount,
      },
      $translation,
    ),
  );

  const regionHovered: ExpressionSpecification = [
    "boolean",
    ["feature-state", "hover"],
    false,
  ];
  const regionSelected: ExpressionSpecification = [
    "boolean",
    ["feature-state", "selected"],
    false,
  ];

  function regionFillOpacity(): ExpressionSpecification {
    return [
      "case",
      regionSelected,
      lowResource ? 0.12 : 0.2,
      regionHovered,
      lowResource ? 0.07 : 0.14,
      lowResource ? 0.008 : 0.018,
    ];
  }

  function regionHaloWidth(): ExpressionSpecification {
    return [
      "case",
      regionHovered,
      lowResource ? 3 : 7,
      regionSelected,
      lowResource ? 2.5 : 5,
      0,
    ];
  }

  function regionMotionDuration(): number {
    return lowResource || prefersReducedMotion ? 0 : 160;
  }

  function weatherEffectRadius(pulse: number): ExpressionSpecification {
    const detailScale = 0.72 + weatherPolicy.particleScale * 0.28;
    return [
      "interpolate",
      ["linear"],
      ["zoom"],
      1,
      (14 + pulse * 6) * detailScale,
      8,
      (38 + pulse * 18) * detailScale,
    ];
  }

  function rendererFailureReason(error: unknown): string {
    if (error instanceof Error && error.message.trim()) {
      return error.message.trim().slice(0, 180);
    }
    return "The detailed weather renderer could not be initialized.";
  }

  function disposeWeatherRenderer(): void {
    weatherRendererGeneration += 1;
    weatherRendererInitializationKey = undefined;
    try {
      weatherRenderer?.dispose();
    } catch {
      // The MapLibre fallback must still recover after a failed GPU teardown.
    }
    weatherRenderer = undefined;
  }

  function renderThreeWeather(timeMs: number): void {
    if (!map || !weatherRenderer || !threeWeatherActive) return;
    const width = mapContainer.clientWidth;
    const height = mapContainer.clientHeight;
    if (width < 1 || height < 1) return;
    try {
      const centre = map.getCenter();
      weatherRenderer.render({
        width,
        height,
        pixelRatio: window.devicePixelRatio || 1,
        zoom: map.getZoom(),
        bearing: map.getBearing(),
        projectionKey: [
          map.getProjection().type,
          width,
          height,
          map.getZoom().toFixed(3),
          map.getBearing().toFixed(2),
          map.getPitch().toFixed(2),
          centre.lng.toFixed(4),
          centre.lat.toFixed(4),
        ].join(":"),
        timeMs,
        project: (longitude, latitude) => {
          const atlasMap = map;
          if (!atlasMap) {
            return { x: -10_000, y: -10_000, surfaceVisibility: 0 };
          }
          const point = atlasMap.project([longitude, latitude]);
          const roundTrip = atlasMap.unproject(point);
          return {
            x: point.x,
            y: point.y,
            surfaceVisibility: weatherProjectionSurfaceVisibility(
              { longitude, latitude },
              { longitude: roundTrip.lng, latitude: roundTrip.lat },
            ),
          };
        },
        surfaceVisibilityAt: (x, y) => {
          const atlasMap = map;
          if (!atlasMap || x < 0 || y < 0 || x > width || y > height) {
            return 0;
          }
          try {
            const geographic = atlasMap.unproject([x, y]);
            const roundTrip = atlasMap.project(geographic);
            return weatherScreenSurfaceVisibility(
              { x, y },
              { x: roundTrip.x, y: roundTrip.y },
            );
          } catch {
            return 0;
          }
        },
      });
    } catch (error) {
      weatherRendererFailureKey = weatherPolicy.profile;
      disposeWeatherRenderer();
      weatherRendererStatus = {
        state: "unavailable",
        reason: rendererFailureReason(error),
      };
      updateAtlas();
    }
  }

  function handleWeatherDeviceLoss(
    backend: WeatherRendererBackend,
    reason: string,
  ): void {
    weatherRendererFailureKey = weatherPolicy.profile;
    queueMicrotask(() => {
      disposeWeatherRenderer();
      weatherRendererStatus = { state: "device_lost", backend, reason };
      updateAtlas();
    });
  }

  function handleWeatherQualityChange(quality: AdaptiveWeatherQuality): void {
    if (weatherRendererStatus.state !== "ready") return;
    weatherRendererStatus = { ...weatherRendererStatus, quality };
  }

  async function synchronizeWeatherRenderer(): Promise<void> {
    const requested =
      mapReady &&
      weatherPolicy.atmosphere &&
      visibleWeatherRenderScene.cells.length > 0;
    if (!requested) {
      weatherRendererFailureKey = undefined;
      if (weatherRenderer || weatherRendererStatus.state !== "disabled") {
        disposeWeatherRenderer();
        weatherRendererStatus = { state: "disabled" };
      }
      return;
    }

    const update = { scene: visibleWeatherRenderScene, policy: weatherPolicy };
    if (weatherRenderer) {
      weatherRenderer.update(update);
      renderThreeWeather(performance.now());
      return;
    }
    const requestKey = weatherPolicy.profile;
    if (
      weatherRendererInitializationKey === requestKey ||
      weatherRendererFailureKey === requestKey
    ) {
      return;
    }

    const generation = ++weatherRendererGeneration;
    weatherRendererInitializationKey = requestKey;
    weatherRendererStatus = { state: "initializing" };
    try {
      const { createThreeWeatherRenderer } =
        await import("$lib/weather/renderer/threeWeatherRenderer");
      const renderer = await createThreeWeatherRenderer(
        weatherCanvas,
        update,
        handleWeatherDeviceLoss,
        handleWeatherQualityChange,
      );
      if (generation !== weatherRendererGeneration) {
        renderer.dispose();
        return;
      }
      weatherRenderer = renderer;
      weatherRendererInitializationKey = undefined;
      weatherRendererStatus = {
        state: "ready",
        backend: renderer.backend,
        quality: renderer.quality,
      };
      updateAtlas();
      renderThreeWeather(performance.now());
    } catch (error) {
      if (generation !== weatherRendererGeneration) return;
      weatherRendererInitializationKey = undefined;
      weatherRendererFailureKey = requestKey;
      weatherRendererStatus = {
        state: "unavailable",
        reason: rendererFailureReason(error),
      };
      updateAtlas();
    }
  }

  function stopWeatherAnimation(): void {
    if (weatherAnimationFrame !== undefined) {
      window.cancelAnimationFrame(weatherAnimationFrame);
      weatherAnimationFrame = undefined;
    }
  }

  function applyStaticWeatherEffect(): void {
    if (!map) return;
    if (map.getLayer(WEATHER_EFFECT_LAYER_ID)) {
      map.setPaintProperty(
        WEATHER_EFFECT_LAYER_ID,
        "circle-radius",
        weatherEffectRadius(0.35),
      );
      map.setPaintProperty(WEATHER_EFFECT_LAYER_ID, "circle-opacity", 0.26);
    }
    for (const layerId of [
      WEATHER_PRECIPITATION_LAYER_ID,
      PLUGIN_WEATHER_PRECIPITATION_LAYER_ID,
    ]) {
      if (map.getLayer(layerId)) {
        map.setPaintProperty(layerId, "text-translate", [0, 0]);
      }
    }
    for (const layerId of [
      WEATHER_CLOUD_HIGHLIGHT_LAYER_ID,
      PLUGIN_WEATHER_CLOUD_HIGHLIGHT_LAYER_ID,
    ]) {
      if (map.getLayer(layerId)) {
        map.setPaintProperty(layerId, "circle-translate", [-5, -5]);
      }
    }
    if (map.getLayer(WEATHER_DUST_LAYER_ID)) {
      map.setPaintProperty(WEATHER_DUST_LAYER_ID, "circle-translate", [0, 0]);
    }
    const flashOpacity = weatherPolicy.lightning ? 0.035 : 0;
    for (const layerId of [
      WEATHER_LIGHTNING_FLASH_LAYER_ID,
      PLUGIN_WEATHER_LIGHTNING_FLASH_LAYER_ID,
    ]) {
      if (map.getLayer(layerId)) {
        map.setPaintProperty(layerId, "circle-opacity", flashOpacity);
      }
    }
    renderThreeWeather(performance.now());
  }

  function updateWeatherAnimation(hasWeatherEffects: boolean): void {
    if (
      !map ||
      (!weatherVisible && !pluginWeatherVisible) ||
      !weatherPolicy.animation ||
      !hasWeatherEffects
    ) {
      stopWeatherAnimation();
      applyStaticWeatherEffect();
      return;
    }
    if (weatherAnimationFrame !== undefined) return;

    const animate = (time: number) => {
      weatherAnimationFrame = window.requestAnimationFrame(animate);
      if (!map || time - weatherAnimationTime < 1000 / weatherPolicy.frameRate)
        return;
      weatherAnimationTime = time;
      if (threeWeatherActive) {
        renderThreeWeather(time);
        return;
      }
      const pulse = (Math.sin(time / 620) + 1) / 2;
      const flow = (time % 900) / 900;
      if (map.getLayer(WEATHER_EFFECT_LAYER_ID)) {
        map.setPaintProperty(
          WEATHER_EFFECT_LAYER_ID,
          "circle-radius",
          weatherEffectRadius(pulse),
        );
        map.setPaintProperty(
          WEATHER_EFFECT_LAYER_ID,
          "circle-opacity",
          0.16 + pulse * 0.2,
        );
      }
      for (const layerId of [
        WEATHER_PRECIPITATION_LAYER_ID,
        PLUGIN_WEATHER_PRECIPITATION_LAYER_ID,
      ]) {
        if (map.getLayer(layerId)) {
          map.setPaintProperty(layerId, "text-translate", [0, flow * 15 - 7]);
        }
      }
      const cloudDrift = Math.sin(time / 2_800) * 3;
      for (const layerId of [
        WEATHER_CLOUD_HIGHLIGHT_LAYER_ID,
        PLUGIN_WEATHER_CLOUD_HIGHLIGHT_LAYER_ID,
      ]) {
        if (map.getLayer(layerId)) {
          map.setPaintProperty(layerId, "circle-translate", [
            -5 + cloudDrift,
            -5,
          ]);
        }
      }
      if (map.getLayer(WEATHER_DUST_LAYER_ID)) {
        map.setPaintProperty(WEATHER_DUST_LAYER_ID, "circle-translate", [
          Math.sin(time / 1_450) * 5,
          Math.cos(time / 1_900) * 3,
        ]);
      }
      const flashOpacity = weatherPolicy.lightningFlashes
        ? lightningFlashOpacity(time)
        : 0.035;
      for (const layerId of [
        WEATHER_LIGHTNING_FLASH_LAYER_ID,
        PLUGIN_WEATHER_LIGHTNING_FLASH_LAYER_ID,
      ]) {
        if (map.getLayer(layerId)) {
          map.setPaintProperty(layerId, "circle-opacity", flashOpacity);
        }
      }
    };
    weatherAnimationFrame = window.requestAnimationFrame(animate);
  }

  function clearHoveredRegion(): void {
    if (map && hoveredRegionFeatureId !== undefined) {
      map.setFeatureState(
        { source: REGION_SOURCE_ID, id: hoveredRegionFeatureId },
        { hover: false },
      );
    }
    if (hoveredRegionFeatureId !== undefined) onhoverregion(undefined);
    hoveredRegionFeatureId = undefined;
  }

  function updateSelectedRegionState(): void {
    if (!map?.getSource(REGION_SOURCE_ID)) return;
    if (
      selectedRegionFeatureId !== undefined &&
      selectedRegionFeatureId !== selectedRegionId
    ) {
      map.setFeatureState(
        { source: REGION_SOURCE_ID, id: selectedRegionFeatureId },
        { selected: false },
      );
    }
    if (selectedRegionId) {
      map.setFeatureState(
        { source: REGION_SOURCE_ID, id: selectedRegionId },
        { selected: true },
      );
    }
    selectedRegionFeatureId = selectedRegionId;
  }
  let handledFocusRequestId = 0;

  type FleetFeatureCollection = {
    type: "FeatureCollection";
    features: Array<{
      type: "Feature";
      geometry: { type: "Point"; coordinates: [number, number] };
      properties: {
        id: string;
        registration: string | null;
        model: string | null;
      };
    }>;
  };

  type FboFeatureCollection = {
    type: "FeatureCollection";
    features: Array<{
      type: "Feature";
      geometry: { type: "Point"; coordinates: [number, number] };
      properties: { id: string; name: string | null; icao: string | null };
    }>;
  };

  type PluginFeatureCollection = {
    type: "FeatureCollection";
    features: Array<{
      type: "Feature";
      geometry: { type: "Point"; coordinates: [number, number] };
      properties: {
        id: string;
        label: string;
        plugin_id: string;
        layer_title: string;
      };
    }>;
  };

  function fleetFeatures(): FleetFeatureCollection {
    return {
      type: "FeatureCollection",
      features: aircraft.flatMap((item) =>
        item.location
          ? [
              {
                type: "Feature" as const,
                geometry: {
                  type: "Point" as const,
                  coordinates: [
                    item.location.longitude,
                    item.location.latitude,
                  ] as [number, number],
                },
                properties: {
                  id: item.id,
                  registration: item.registration,
                  model: item.model,
                },
              },
            ]
          : [],
      ),
    };
  }

  function fboFeatures(): FboFeatureCollection {
    return {
      type: "FeatureCollection",
      features: fbos.flatMap((item) =>
        item.airport?.location
          ? [
              {
                type: "Feature" as const,
                geometry: {
                  type: "Point" as const,
                  coordinates: [
                    item.airport.location.longitude,
                    item.airport.location.latitude,
                  ] as [number, number],
                },
                properties: {
                  id: item.id,
                  name: item.name,
                  icao: item.airport.icao,
                },
              },
            ]
          : [],
      ),
    };
  }

  function pluginFeatures(): PluginFeatureCollection {
    return {
      type: "FeatureCollection",
      features: pluginLayers.flatMap((published) =>
        published.layer.points.map((point) => ({
          type: "Feature" as const,
          geometry: {
            type: "Point" as const,
            coordinates: [
              point.location.longitude,
              point.location.latitude,
            ] as [number, number],
          },
          properties: {
            id: `${published.plugin_id}:${published.layer.id}:${point.id}`,
            label: point.label,
            plugin_id: published.plugin_id,
            layer_title: published.layer.title,
          },
        })),
      ),
    };
  }

  function pluginWeatherColor(): ExpressionSpecification {
    return [
      "match",
      ["get", "condition"],
      "clear",
      $activeTheme.colors.success,
      "cloud",
      $activeTheme.colors.text_muted,
      "rain",
      $activeTheme.colors.highlight,
      "snow",
      $activeTheme.colors.text,
      "convective",
      $activeTheme.colors.danger,
      "obscuration",
      $activeTheme.colors.accent,
      $activeTheme.colors.line,
    ];
  }

  function weatherCoverageColor(
    property: "condition" | "effect",
  ): ExpressionSpecification {
    return [
      "match",
      ["get", property],
      "cloud",
      WEATHER_ZONE_COLORS.cloud,
      "rain",
      WEATHER_ZONE_COLORS.rain,
      "snow",
      WEATHER_ZONE_COLORS.snow,
      "convective",
      WEATHER_ZONE_COLORS.convective,
      "obscuration",
      WEATHER_ZONE_COLORS.obscuration,
      "dust",
      WEATHER_ZONE_COLORS.dust,
      "#8498a6",
    ];
  }

  function registerWeatherZonePatterns(atlasMap: MapLibreMap): void {
    for (const pattern of weatherZonePatternImages()) {
      if (atlasMap.hasImage(pattern.id)) continue;
      atlasMap.addImage(pattern.id, pattern.image, { pixelRatio: 2 });
    }
  }

  function resolvedDaylightTime(): Date {
    const selected = daylightAt ? new Date(daylightAt) : new Date();
    return Number.isFinite(selected.getTime()) ? selected : new Date();
  }

  function synchronizeDaylightSource(): void {
    if (!map || !mapReady) return;
    const selected = resolvedDaylightTime();
    const timeKey = daylightAt
      ? selected.toISOString()
      : String(Math.floor(selected.getTime() / 60_000));
    const signature = `${timeKey}:${lowResource}`;
    if (signature === daylightSourceSignature) return;
    (map.getSource(DAYLIGHT_SOURCE_ID) as GeoJSONSource | undefined)?.setData(
      daylightFeatureCollection(selected, lowResource ? 90 : 180),
    );
    daylightSourceSignature = signature;
  }

  function pluginRadarMapId(frameId: string): string {
    return `${PLUGIN_RADAR_PREFIX}-${frameId.replaceAll(/[^A-Za-z0-9_-]/g, "-")}`;
  }

  function syncPluginRadarFrames(): void {
    if (!map || !mapReady) return;
    const nextLayerIds = new Set<string>();
    const nextFrameVersions = new Map<string, string>();
    for (const frame of activeRadarFrames) {
      for (const tile of frame.tiles) {
        const mapId = pluginRadarMapId(tile.id);
        syncRadarImageLayer(
          `${mapId}-source`,
          `${mapId}-layer`,
          tile.url,
          tile.coordinates,
          frame.frame_time,
          {
            "raster-opacity": lowResource ? 0.42 : 0.58,
            "raster-fade-duration": prefersReducedMotion ? 0 : 260,
          },
          nextLayerIds,
          nextFrameVersions,
        );
        if (tile.coverage_url) {
          syncRadarImageLayer(
            `${mapId}-coverage-source`,
            `${mapId}-coverage-layer`,
            tile.coverage_url,
            tile.coordinates,
            `${frame.frame_time}:coverage`,
            {
              "raster-opacity": lowResource ? 0.38 : 0.52,
              "raster-fade-duration": 0,
              "raster-saturation": -1,
              "raster-brightness-min": 0.22,
              "raster-brightness-max": 0.48,
            },
            nextLayerIds,
            nextFrameVersions,
          );
        }
      }
    }
    for (const layerId of pluginRadarLayerIds) {
      if (nextLayerIds.has(layerId)) continue;
      const sourceId = `${layerId.slice(0, -"-layer".length)}-source`;
      if (map.getLayer(layerId)) map.removeLayer(layerId);
      if (map.getSource(sourceId)) map.removeSource(sourceId);
    }
    pluginRadarLayerIds = nextLayerIds;
    pluginRadarFrameVersions = nextFrameVersions;
  }

  function syncRadarImageLayer(
    sourceId: string,
    layerId: string,
    url: string,
    coordinates: [
      [number, number],
      [number, number],
      [number, number],
      [number, number],
    ],
    version: string,
    paint: Record<string, number>,
    nextLayerIds: Set<string>,
    nextFrameVersions: Map<string, string>,
  ): void {
    if (!map) return;
    nextLayerIds.add(layerId);
    nextFrameVersions.set(layerId, version);
    const image = map.getSource(sourceId) as ImageSource | undefined;
    if (image && pluginRadarFrameVersions.get(layerId) !== version) {
      image.updateImage({ url, coordinates });
    } else if (!image) {
      map.addSource(sourceId, { type: "image", url, coordinates });
      map.addLayer(
        {
          id: layerId,
          type: "raster",
          source: sourceId,
          layout: { visibility: pluginWeatherVisible ? "visible" : "none" },
          paint,
        },
        PLUGIN_WEATHER_ATMOSPHERE_LAYER_ID,
      );
    }
    map.setLayoutProperty(
      layerId,
      "visibility",
      pluginWeatherVisible ? "visible" : "none",
    );
  }

  function selectPreviousRadarFrame(): void {
    radarPlaying = false;
    radarFrameIndex = Math.max(0, radarFrameIndex - 1);
  }

  function selectNextRadarFrame(): void {
    radarPlaying = false;
    radarFrameIndex = Math.min(radarFrameCount - 1, radarFrameIndex + 1);
  }

  function toggleRadarPlayback(): void {
    if (radarStaticPresentation || radarFrameCount <= 1) return;
    radarPlaying = !radarPlaying;
  }

  function formatRadarFrameTime(value: string): string {
    const date = new Date(value);
    return Number.isFinite(date.getTime())
      ? new Intl.DateTimeFormat(undefined, {
          year: "numeric",
          month: "short",
          day: "numeric",
          hour: "2-digit",
          minute: "2-digit",
          timeZoneName: "short",
        }).format(date)
      : value;
  }

  function updateAtlas(): void {
    if (!map || !mapReady) return;

    synchronizeDaylightSource();
    const daylightVisibility = daylightVisible ? "visible" : "none";
    map.setLayoutProperty(
      DAYLIGHT_SHADE_LAYER_ID,
      "visibility",
      daylightVisibility,
    );
    map.setLayoutProperty(
      DAYLIGHT_TERMINATOR_LAYER_ID,
      "visibility",
      daylightVisibility,
    );

    const regionVisibility = regionsVisible ? "visible" : "none";
    map.setLayoutProperty(REGION_FILL_LAYER_ID, "visibility", regionVisibility);
    map.setLayoutProperty(
      REGION_BOUNDARY_LAYER_ID,
      "visibility",
      regionVisibility,
    );
    map.setLayoutProperty(REGION_HALO_LAYER_ID, "visibility", regionVisibility);
    for (const layerId of REGION_LABEL_LAYER_IDS) {
      map.setLayoutProperty(layerId, "visibility", regionVisibility);
    }
    if (!regionsVisible) clearHoveredRegion();
    updateSelectedRegionState();
    map.setPaintProperty(
      REGION_FILL_LAYER_ID,
      "fill-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(
      REGION_FILL_LAYER_ID,
      "fill-opacity",
      regionFillOpacity(),
    );
    map.setPaintProperty(
      REGION_BOUNDARY_LAYER_ID,
      "line-color",
      $activeTheme.colors.map_label,
    );
    map.setPaintProperty(
      REGION_HALO_LAYER_ID,
      "line-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(REGION_HALO_LAYER_ID, "line-width", regionHaloWidth());
    for (const layerId of REGION_LABEL_LAYER_IDS) {
      map.setPaintProperty(
        layerId,
        "text-color",
        $activeTheme.colors.map_label,
      );
      map.setPaintProperty(
        layerId,
        "text-halo-color",
        $activeTheme.colors.map_halo,
      );
    }

    const fleet = fleetFeatures();
    const fboNetwork = fboFeatures();
    const pluginData = pluginFeatures();
    const pluginWeatherData = pluginWeatherGridFeatures(pluginWeatherLayers);
    const pluginWeatherCoverageData =
      pluginWeatherGridCoverageFeatures(pluginWeatherLayers);
    const pluginRadarCoverageData =
      pluginRadarCoverageFeatures(pluginWeatherLayers);
    const routes = routeLineFeatures(flightRoute);
    const routeMarkers = routeMarkerFeatures(flightRoute);
    const weatherStations = weatherStationFeatures(weather);
    const weatherWinds = weatherWindFeatures(weather);
    const dispatchRouteData = atlasRouteGeoJson(route);
    const routeWeatherData = routeWeatherLineFeatures(routeWeather);
    (map.getSource(FLEET_SOURCE_ID) as GeoJSONSource | undefined)?.setData(
      fleet,
    );
    (map.getSource(FBO_SOURCE_ID) as GeoJSONSource | undefined)?.setData(
      fboNetwork,
    );
    (map.getSource(PLUGIN_SOURCE_ID) as GeoJSONSource | undefined)?.setData(
      pluginData,
    );
    (
      map.getSource(PLUGIN_WEATHER_GRID_SOURCE_ID) as GeoJSONSource | undefined
    )?.setData(pluginWeatherData);
    (
      map.getSource(PLUGIN_WEATHER_COVERAGE_SOURCE_ID) as
        GeoJSONSource | undefined
    )?.setData(pluginWeatherCoverageData);
    (
      map.getSource(PLUGIN_RADAR_COVERAGE_SOURCE_ID) as
        GeoJSONSource | undefined
    )?.setData(pluginRadarCoverageData);
    syncPluginRadarFrames();
    (map.getSource(ROUTE_SOURCE_ID) as GeoJSONSource | undefined)?.setData(
      routes,
    );
    (
      map.getSource(ROUTE_MARKER_SOURCE_ID) as GeoJSONSource | undefined
    )?.setData(routeMarkers);
    (map.getSource(WEATHER_SOURCE_ID) as GeoJSONSource | undefined)?.setData(
      weatherStations,
    );
    (
      map.getSource(WEATHER_WIND_SOURCE_ID) as GeoJSONSource | undefined
    )?.setData(weatherWinds);
    (
      map.getSource(DISPATCH_ROUTE_SOURCE_ID) as GeoJSONSource | undefined
    )?.setData(dispatchRouteData);
    (
      map.getSource(ROUTE_WEATHER_SOURCE_ID) as GeoJSONSource | undefined
    )?.setData(routeWeatherData);

    const visibility = fleetVisible ? "visible" : "none";
    map.setLayoutProperty(FLEET_LAYER_ID, "visibility", visibility);
    map.setLayoutProperty(FLEET_LABEL_LAYER_ID, "visibility", visibility);
    const fboVisibility = fboVisible ? "visible" : "none";
    map.setLayoutProperty(FBO_LAYER_ID, "visibility", fboVisibility);
    map.setLayoutProperty(FBO_LABEL_LAYER_ID, "visibility", fboVisibility);
    const pluginVisibility = pluginLayersVisible ? "visible" : "none";
    map.setLayoutProperty(PLUGIN_LAYER_ID, "visibility", pluginVisibility);
    map.setLayoutProperty(
      PLUGIN_LABEL_LAYER_ID,
      "visibility",
      pluginVisibility,
    );
    const pluginWeatherVisibility = pluginWeatherVisible ? "visible" : "none";
    const pluginWeatherCoverageVisibility =
      weatherCoverageVisible && pluginWeatherVisible ? "visible" : "none";
    for (const layerId of [
      PLUGIN_WEATHER_COVERAGE_FILL_LAYER_ID,
      PLUGIN_WEATHER_COVERAGE_PATTERN_LAYER_ID,
      PLUGIN_WEATHER_COVERAGE_LINE_LAYER_ID,
      PLUGIN_RADAR_COVERAGE_FILL_LAYER_ID,
      PLUGIN_RADAR_COVERAGE_PATTERN_LAYER_ID,
      PLUGIN_RADAR_COVERAGE_LINE_LAYER_ID,
    ]) {
      map.setLayoutProperty(
        layerId,
        "visibility",
        pluginWeatherCoverageVisibility,
      );
    }
    map.setLayoutProperty(
      PLUGIN_WEATHER_GRID_LAYER_ID,
      "visibility",
      pluginWeatherVisibility,
    );
    map.setLayoutProperty(
      PLUGIN_WEATHER_ATMOSPHERE_LAYER_ID,
      "visibility",
      pluginWeatherVisible &&
        weatherPolicy.atmosphere &&
        mapWeatherEffectsActive
        ? "visible"
        : "none",
    );
    map.setLayoutProperty(
      PLUGIN_WEATHER_CLOUD_HIGHLIGHT_LAYER_ID,
      "visibility",
      pluginWeatherVisible &&
        weatherPolicy.clouds &&
        mapWeatherEffectsActive &&
        weatherPolicy.profile === "cinematic"
        ? "visible"
        : "none",
    );
    map.setLayoutProperty(
      PLUGIN_WEATHER_PRECIPITATION_LAYER_ID,
      "visibility",
      pluginWeatherVisible &&
        weatherPolicy.precipitation &&
        mapWeatherEffectsActive
        ? "visible"
        : "none",
    );
    for (const layerId of [
      PLUGIN_WEATHER_LIGHTNING_FLASH_LAYER_ID,
      PLUGIN_WEATHER_LIGHTNING_LAYER_ID,
    ]) {
      map.setLayoutProperty(
        layerId,
        "visibility",
        pluginWeatherVisible &&
          weatherPolicy.lightning &&
          mapWeatherEffectsActive
          ? "visible"
          : "none",
      );
    }
    const weatherVisibility = weatherVisible ? "visible" : "none";
    const weatherCoverageVisibility =
      weatherCoverageVisible && weatherVisible ? "visible" : "none";
    map.setLayoutProperty(
      WEATHER_COVERAGE_OUTER_LAYER_ID,
      "visibility",
      weatherCoverageVisibility,
    );
    map.setLayoutProperty(
      WEATHER_COVERAGE_INNER_LAYER_ID,
      "visibility",
      weatherCoverageVisibility,
    );
    map.setLayoutProperty(
      WEATHER_COVERAGE_PATTERN_LAYER_ID,
      "visibility",
      weatherCoverageVisibility,
    );
    const gpuWeatherVisibility =
      weatherVisible && weatherPolicy.atmosphere && mapWeatherEffectsActive
        ? "visible"
        : "none";
    map.setLayoutProperty(
      WEATHER_ATMOSPHERE_LAYER_ID,
      "visibility",
      gpuWeatherVisibility,
    );
    const cloudVisibility =
      weatherVisible && weatherPolicy.clouds && mapWeatherEffectsActive
        ? "visible"
        : "none";
    map.setLayoutProperty(
      WEATHER_CLOUD_SHADOW_LAYER_ID,
      "visibility",
      cloudVisibility,
    );
    map.setLayoutProperty(
      WEATHER_CLOUD_BODY_LAYER_ID,
      "visibility",
      cloudVisibility,
    );
    map.setLayoutProperty(
      WEATHER_CLOUD_HIGHLIGHT_LAYER_ID,
      "visibility",
      weatherVisible &&
        weatherPolicy.clouds &&
        mapWeatherEffectsActive &&
        weatherPolicy.profile === "cinematic"
        ? "visible"
        : "none",
    );
    map.setLayoutProperty(
      WEATHER_PRECIPITATION_LAYER_ID,
      "visibility",
      weatherVisible && weatherPolicy.precipitation && mapWeatherEffectsActive
        ? "visible"
        : "none",
    );
    for (const layerId of [
      WEATHER_LIGHTNING_FLASH_LAYER_ID,
      WEATHER_LIGHTNING_LAYER_ID,
    ]) {
      map.setLayoutProperty(
        layerId,
        "visibility",
        weatherVisible && weatherPolicy.lightning && mapWeatherEffectsActive
          ? "visible"
          : "none",
      );
    }
    map.setLayoutProperty(
      WEATHER_DUST_LAYER_ID,
      "visibility",
      weatherVisible && weatherPolicy.dust && mapWeatherEffectsActive
        ? "visible"
        : "none",
    );
    map.setLayoutProperty(
      WEATHER_DUST_CORE_LAYER_ID,
      "visibility",
      weatherVisible &&
        weatherPolicy.dust &&
        mapWeatherEffectsActive &&
        weatherPolicy.profile === "cinematic"
        ? "visible"
        : "none",
    );
    map.setLayoutProperty(
      WEATHER_EFFECT_LAYER_ID,
      "visibility",
      gpuWeatherVisibility,
    );
    map.setLayoutProperty(
      WEATHER_WIND_LAYER_ID,
      "visibility",
      weatherVisible && weatherPolicy.atmosphere ? "visible" : "none",
    );
    map.setLayoutProperty(
      WEATHER_WIND_TIP_LAYER_ID,
      "visibility",
      weatherVisible && weatherPolicy.atmosphere ? "visible" : "none",
    );
    map.setLayoutProperty(WEATHER_LAYER_ID, "visibility", weatherVisibility);
    map.setLayoutProperty(
      WEATHER_LABEL_LAYER_ID,
      "visibility",
      weatherVisibility,
    );
    const routeVisibility = routeVisible ? "visible" : "none";
    const routeWeatherVisibility =
      routeVisible && pluginWeatherVisible && routeWeather ? "visible" : "none";
    for (const layerId of [
      ROUTE_WEATHER_HALO_LAYER_ID,
      ROUTE_WEATHER_SUPPORTED_LAYER_ID,
      ROUTE_WEATHER_CURRENT_CONTEXT_LAYER_ID,
      ROUTE_WEATHER_UNAVAILABLE_LAYER_ID,
    ]) {
      map.setLayoutProperty(layerId, "visibility", routeWeatherVisibility);
    }
    map.setLayoutProperty(
      DISPATCH_ROUTE_LINE_LAYER_ID,
      "visibility",
      routeVisibility,
    );
    map.setLayoutProperty(
      DISPATCH_ROUTE_POINT_LAYER_ID,
      "visibility",
      routeVisibility,
    );
    map.setLayoutProperty(
      DISPATCH_ROUTE_LABEL_LAYER_ID,
      "visibility",
      routeVisibility,
    );
    map.setPaintProperty(FLEET_LAYER_ID, "circle-color", [
      "case",
      ["==", ["get", "id"], selectedAircraftId ?? ""],
      $activeTheme.colors.highlight,
      $activeTheme.colors.map_aircraft,
    ]);
    map.setPaintProperty(FBO_LAYER_ID, "circle-color", [
      "case",
      ["==", ["get", "id"], selectedFboId ?? ""],
      $activeTheme.colors.highlight,
      $activeTheme.colors.map_fbo,
    ]);
    map.setPaintProperty(
      FLEET_LAYER_ID,
      "circle-stroke-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      FBO_LAYER_ID,
      "circle-stroke-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      FLEET_LABEL_LAYER_ID,
      "text-color",
      $activeTheme.colors.map_label,
    );
    map.setPaintProperty(
      FBO_LABEL_LAYER_ID,
      "text-color",
      $activeTheme.colors.map_label,
    );
    map.setPaintProperty(
      FLEET_LABEL_LAYER_ID,
      "text-halo-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      FBO_LABEL_LAYER_ID,
      "text-halo-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      PLUGIN_LAYER_ID,
      "circle-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(
      PLUGIN_LAYER_ID,
      "circle-stroke-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(
      PLUGIN_WEATHER_ATMOSPHERE_LAYER_ID,
      "circle-color",
      pluginWeatherColor(),
    );
    map.setPaintProperty(
      PLUGIN_WEATHER_CLOUD_HIGHLIGHT_LAYER_ID,
      "circle-color",
      $activeTheme.colors.text,
    );
    map.setPaintProperty(
      PLUGIN_WEATHER_PRECIPITATION_LAYER_ID,
      "text-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(
      PLUGIN_WEATHER_LIGHTNING_FLASH_LAYER_ID,
      "circle-color",
      LIGHTNING_COLOR,
    );
    map.setPaintProperty(
      PLUGIN_WEATHER_LIGHTNING_LAYER_ID,
      "text-halo-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      PLUGIN_WEATHER_GRID_LAYER_ID,
      "circle-color",
      pluginWeatherColor(),
    );
    map.setPaintProperty(
      PLUGIN_WEATHER_GRID_LAYER_ID,
      "circle-stroke-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      PLUGIN_LABEL_LAYER_ID,
      "text-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(
      PLUGIN_LABEL_LAYER_ID,
      "text-halo-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      DISPATCH_ROUTE_LINE_LAYER_ID,
      "line-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(DISPATCH_ROUTE_POINT_LAYER_ID, "circle-color", [
      "case",
      ["==", ["get", "id"], selectedRouteFeatureId ?? ""],
      $activeTheme.colors.accent,
      ["==", ["get", "kind"], "alternate"],
      $activeTheme.colors.map_fbo,
      $activeTheme.colors.highlight,
    ]);
    map.setPaintProperty(
      DISPATCH_ROUTE_POINT_LAYER_ID,
      "circle-stroke-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      DISPATCH_ROUTE_LABEL_LAYER_ID,
      "text-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(
      DISPATCH_ROUTE_LABEL_LAYER_ID,
      "text-halo-color",
      $activeTheme.colors.map_halo,
    );

    map.setPaintProperty(
      PLANNED_ROUTE_LAYER_ID,
      "line-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(
      RECORDED_ROUTE_LAYER_ID,
      "line-color",
      $activeTheme.colors.accent,
    );
    map.setPaintProperty(ROUTE_MARKER_LAYER_ID, "circle-color", [
      "case",
      ["==", ["get", "id"], selectedRoutePointId ?? ""],
      $activeTheme.colors.accent,
      $activeTheme.colors.highlight,
    ]);
    map.setPaintProperty(
      ROUTE_MARKER_LAYER_ID,
      "circle-stroke-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      ROUTE_LABEL_LAYER_ID,
      "text-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(
      ROUTE_LABEL_LAYER_ID,
      "text-halo-color",
      $activeTheme.colors.map_halo,
    );

    map.setPaintProperty(WEATHER_LAYER_ID, "circle-color", [
      "case",
      ["==", ["get", "id"], selectedWeatherStationId ?? ""],
      $activeTheme.colors.text,
      ["==", ["get", "category"], "vfr"],
      $activeTheme.colors.success,
      ["==", ["get", "category"], "mvfr"],
      $activeTheme.colors.highlight,
      ["==", ["get", "category"], "ifr"],
      $activeTheme.colors.accent,
      ["==", ["get", "category"], "lifr"],
      $activeTheme.colors.danger,
      $activeTheme.colors.text_muted,
    ]);
    map.setPaintProperty(
      WEATHER_LAYER_ID,
      "circle-stroke-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      WEATHER_LABEL_LAYER_ID,
      "text-color",
      $activeTheme.colors.map_label,
    );
    map.setPaintProperty(
      WEATHER_LABEL_LAYER_ID,
      "text-halo-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(WEATHER_ATMOSPHERE_LAYER_ID, "heatmap-color", [
      "interpolate",
      ["linear"],
      ["heatmap-density"],
      0,
      "rgba(0, 0, 0, 0)",
      0.15,
      $activeTheme.colors.success,
      0.42,
      $activeTheme.colors.highlight,
      0.72,
      $activeTheme.colors.accent,
      1,
      $activeTheme.colors.danger,
    ]);
    map.setPaintProperty(
      WEATHER_CLOUD_SHADOW_LAYER_ID,
      "circle-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      WEATHER_CLOUD_BODY_LAYER_ID,
      "circle-color",
      $activeTheme.colors.text_muted,
    );
    map.setPaintProperty(
      WEATHER_CLOUD_HIGHLIGHT_LAYER_ID,
      "circle-color",
      $activeTheme.colors.text,
    );
    map.setPaintProperty(WEATHER_EFFECT_LAYER_ID, "circle-color", [
      "match",
      ["get", "effect"],
      "rain",
      $activeTheme.colors.highlight,
      "snow",
      $activeTheme.colors.map_label,
      "convective",
      $activeTheme.colors.danger,
      "obscuration",
      $activeTheme.colors.text_muted,
      "dust",
      DUST_OUTER_COLOR,
      $activeTheme.colors.accent,
    ]);
    map.setPaintProperty(WEATHER_EFFECT_LAYER_ID, "circle-stroke-color", [
      "match",
      ["get", "effect"],
      "convective",
      $activeTheme.colors.danger,
      $activeTheme.colors.highlight,
    ]);
    map.setPaintProperty(
      WEATHER_PRECIPITATION_LAYER_ID,
      "text-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(
      WEATHER_LIGHTNING_FLASH_LAYER_ID,
      "circle-color",
      LIGHTNING_COLOR,
    );
    map.setPaintProperty(
      WEATHER_LIGHTNING_LAYER_ID,
      "text-halo-color",
      $activeTheme.colors.map_halo,
    );
    map.setPaintProperty(
      WEATHER_DUST_LAYER_ID,
      "circle-color",
      DUST_OUTER_COLOR,
    );
    map.setPaintProperty(
      WEATHER_DUST_CORE_LAYER_ID,
      "circle-color",
      DUST_CORE_COLOR,
    );
    map.setPaintProperty(WEATHER_WIND_LAYER_ID, "line-color", [
      "interpolate",
      ["linear"],
      ["get", "wind_speed_kt"],
      0,
      $activeTheme.colors.success,
      20,
      $activeTheme.colors.highlight,
      40,
      $activeTheme.colors.accent,
      60,
      $activeTheme.colors.danger,
    ]);
    map.setPaintProperty(
      WEATHER_WIND_TIP_LAYER_ID,
      "text-color",
      $activeTheme.colors.highlight,
    );
    map.setPaintProperty(
      WEATHER_WIND_TIP_LAYER_ID,
      "text-halo-color",
      $activeTheme.colors.map_halo,
    );
    updateWeatherAnimation(
      (weatherVisible &&
        weatherStations.features.some(
          ({ properties }) => properties.effect !== "none",
        )) ||
        (pluginWeatherVisible &&
          pluginWeatherData.features.some(
            ({ properties }) =>
              !["clear", "unknown"].includes(properties.condition),
          )),
    );

    const routeSignature = flightRouteSignature(flightRoute);
    const weatherSignature = weatherVisible ? weatherMapSignature(weather) : "";
    const routeViewportSignature =
      routeSignature || weatherSignature
        ? `${routeSignature}|${weatherSignature}|route-focus:${selectedRoutePointId ?? "all"}|weather-focus:${selectedWeatherStationId ?? "all"}`
        : "";
    if (
      routeViewportSignature &&
      routeViewportSignature !== fittedAtlasSignature
    ) {
      const focusedPoint =
        weatherPointCoordinates(weather, selectedWeatherStationId) ??
        routePointCoordinates(flightRoute, selectedRoutePointId);
      fittedAtlasSignature = routeViewportSignature;
      if (focusedPoint) {
        map.easeTo({ center: focusedPoint, zoom: 8, duration: 700 });
        return;
      }
      const coordinates = routeSignature
        ? routeFitCoordinates(flightRoute)
        : weatherFitCoordinates(weather);
      if (coordinates.length === 1) {
        map.easeTo({ center: coordinates[0], zoom: 8, duration: 700 });
      } else if (coordinates.length > 1) {
        const routeBounds = coordinates.reduce(
          (current, coordinate) => current.extend(coordinate),
          new maplibregl.LngLatBounds(coordinates[0], coordinates[0]),
        );
        map.fitBounds(routeBounds, { padding: 90, maxZoom: 8, duration: 700 });
      }
      return;
    }
    if (routeSignature || weatherSignature) return;

    const visibleCoordinates: [number, number][] = [
      ...(fleetVisible
        ? fleet.features.map((feature) => feature.geometry.coordinates)
        : []),
      ...(fboVisible
        ? fboNetwork.features.map((feature) => feature.geometry.coordinates)
        : []),
      ...(pluginLayersVisible
        ? pluginData.features.map((feature) => feature.geometry.coordinates)
        : []),
      ...(routeVisible
        ? dispatchRouteData.features.flatMap((feature) =>
            feature.geometry.type === "Point"
              ? [feature.geometry.coordinates]
              : [],
          )
        : []),
    ];
    const signature = visibleCoordinates
      .map((coordinate) => coordinate.join(","))
      .sort()
      .join("|");
    if (!signature || signature === fittedAtlasSignature) return;

    fittedAtlasSignature = signature;
    const overviewCoordinates = balancedOverviewCoordinates(visibleCoordinates);
    const bounds = overviewCoordinates.reduce(
      (current, coordinate) => current.extend(coordinate),
      new maplibregl.LngLatBounds(
        overviewCoordinates[0],
        overviewCoordinates[0],
      ),
    );
    map.fitBounds(bounds, { padding: 90, maxZoom: 6, duration: 700 });
  }

  function applyFocusRequest(): void {
    if (
      !map ||
      !mapReady ||
      !focusRequest ||
      focusRequest.request_id === handledFocusRequestId
    ) {
      return;
    }
    handledFocusRequestId = focusRequest.request_id;

    if (focusRequest.kind === "route") {
      const bounds = atlasRouteBounds(route);
      if (bounds) {
        map.fitBounds(bounds, { padding: 110, maxZoom: 7, duration: 700 });
      }
      return;
    }

    const feature = findRouteFeature(route, focusRequest.feature_id);
    if (feature?.location) {
      map.flyTo({
        center: [feature.location.longitude, feature.location.latitude],
        zoom: Math.max(map.getZoom(), 7),
        duration: 700,
      });
    }
  }

  let maplibregl: typeof import("maplibre-gl");

  $effect(() => {
    aircraft;
    fbos;
    fleetVisible;
    fboVisible;
    pluginLayers;
    pluginLayersVisible;
    pluginWeatherLayers;
    pluginWeatherVisible;
    flightRoute;
    weather;
    weatherVisible;
    daylightVisible;
    daylightAt;
    weatherCoverageVisible;
    weatherGraphics;
    regionsVisible;
    lowResource;
    selectedRegionId;
    prefersReducedMotion;
    selectedRoutePointId;
    selectedWeatherStationId;
    route;
    routeWeather;
    routeVisible;
    selectedAircraftId;
    selectedFboId;
    selectedRouteFeatureId;
    radarFrameIndex;
    radarPlaying;
    weatherRendererStatus;
    $activeTheme;
    updateAtlas();
  });

  $effect(() => {
    const signature = radarTimelines
      .map((timeline) =>
        timeline.frames.map((frame) => frame.frame_time).join(","),
      )
      .join("|");
    if (signature === radarTimelineSignature) return;
    radarTimelineSignature = signature;
    radarFrameIndex = radarStaticPresentation
      ? Math.max(0, radarFrameCount - 1)
      : 0;
    radarPlaying = !radarStaticPresentation && radarFrameCount > 1;
  });

  $effect(() => {
    mapReady;
    weatherPolicy;
    visibleWeatherRenderScene;
    void synchronizeWeatherRenderer();
  });

  $effect(() => {
    mapReady;
    focusRequest;
    route;
    applyFocusRequest();
  });

  onMount(() => {
    let cancelled = false;
    const daylightRefreshInterval = window.setInterval(() => {
      if (daylightAt) return;
      daylightSourceSignature = "";
      synchronizeDaylightSource();
    }, 60_000);
    const radarAnimationInterval = window.setInterval(() => {
      if (radarStaticPresentation || !radarPlaying || radarFrameCount <= 1) {
        return;
      }
      radarFrameIndex = (radarFrameIndex + 1) % radarFrameCount;
    }, 1_250);
    const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const updateMotionPreference = () => {
      prefersReducedMotion = motionQuery.matches;
    };
    updateMotionPreference();
    motionQuery.addEventListener("change", updateMotionPreference);

    void import("maplibre-gl").then((module) => {
      if (cancelled) return;

      maplibregl = module;
      const atlasMap = new maplibregl.Map({
        container: mapContainer,
        style: "https://demotiles.maplibre.org/globe.json",
        center: initialView
          ? [initialView.longitude, initialView.latitude]
          : ATLAS_HOME_CENTER,
        zoom: initialView?.zoom ?? 1.25,
        bearing: initialView?.bearing ?? 0,
        pitch: initialView?.pitch ?? 0,
        attributionControl: false,
      });
      map = atlasMap;
      atlasMap.on("moveend", () => {
        const centre = atlasMap.getCenter();
        onviewchange({
          longitude: centre.lng,
          latitude: centre.lat,
          zoom: atlasMap.getZoom(),
          bearing: atlasMap.getBearing(),
          pitch: atlasMap.getPitch(),
        });
      });
      atlasMap.on("render", () => {
        if (threeWeatherActive && !weatherPolicy.animation) {
          renderThreeWeather(performance.now());
        }
      });

      atlasMap.addControl(
        new maplibregl.NavigationControl({ visualizePitch: true }),
        "top-right",
      );
      atlasMap.addControl(
        new maplibregl.AttributionControl({ compact: true }),
        "bottom-right",
      );

      atlasMap.on("load", () => {
        registerWeatherZonePatterns(atlasMap);
        atlasMap.addSource(REGION_SOURCE_ID, {
          type: "geojson",
          data: ATLAS_ADMIN1_DATASET_URL,
          promoteId: "region_id",
        });
        atlasMap.addSource(FLEET_SOURCE_ID, {
          type: "geojson",
          data: fleetFeatures(),
        });
        atlasMap.addSource(FBO_SOURCE_ID, {
          type: "geojson",
          data: fboFeatures(),
        });
        atlasMap.addSource(PLUGIN_SOURCE_ID, {
          type: "geojson",
          data: pluginFeatures(),
        });
        atlasMap.addSource(ROUTE_SOURCE_ID, {
          type: "geojson",
          data: routeLineFeatures(flightRoute),
        });
        atlasMap.addSource(ROUTE_MARKER_SOURCE_ID, {
          type: "geojson",
          data: routeMarkerFeatures(flightRoute),
        });
        atlasMap.addSource(WEATHER_SOURCE_ID, {
          type: "geojson",
          data: weatherStationFeatures(weather),
        });
        atlasMap.addSource(WEATHER_WIND_SOURCE_ID, {
          type: "geojson",
          data: weatherWindFeatures(weather),
        });
        atlasMap.addSource(PLUGIN_WEATHER_GRID_SOURCE_ID, {
          type: "geojson",
          data: pluginWeatherGridFeatures(pluginWeatherLayers),
        });
        atlasMap.addSource(PLUGIN_WEATHER_COVERAGE_SOURCE_ID, {
          type: "geojson",
          data: pluginWeatherGridCoverageFeatures(pluginWeatherLayers),
        });
        atlasMap.addSource(PLUGIN_RADAR_COVERAGE_SOURCE_ID, {
          type: "geojson",
          data: pluginRadarCoverageFeatures(pluginWeatherLayers),
        });
        atlasMap.addSource(DAYLIGHT_SOURCE_ID, {
          type: "geojson",
          data: daylightFeatureCollection(
            resolvedDaylightTime(),
            lowResource ? 90 : 180,
          ),
        });
        daylightSourceSignature = "";
        atlasMap.addLayer({
          id: DAYLIGHT_SHADE_LAYER_ID,
          type: "fill",
          source: DAYLIGHT_SOURCE_ID,
          filter: ["==", ["get", "kind"], "shade"],
          layout: { visibility: daylightVisible ? "visible" : "none" },
          paint: {
            "fill-color": [
              "match",
              ["get", "band"],
              "civil_twilight",
              "#334963",
              "nautical_twilight",
              "#203854",
              "astronomical_twilight",
              "#122b49",
              "night",
              "#081b36",
              "#081b36",
            ],
            "fill-opacity": [
              "match",
              ["get", "band"],
              "civil_twilight",
              lowResource ? 0.1 : 0.12,
              "nautical_twilight",
              lowResource ? 0.17 : 0.21,
              "astronomical_twilight",
              lowResource ? 0.25 : 0.31,
              "night",
              lowResource ? 0.34 : 0.43,
              0,
            ],
            "fill-antialias": false,
          },
        });
        atlasMap.addLayer({
          id: DAYLIGHT_TERMINATOR_LAYER_ID,
          type: "line",
          source: DAYLIGHT_SOURCE_ID,
          filter: ["==", ["get", "kind"], "terminator"],
          layout: { visibility: daylightVisible ? "visible" : "none" },
          paint: {
            "line-color": DAYLIGHT_TERMINATOR_COLOR,
            "line-width": ["interpolate", ["linear"], ["zoom"], 1, 0.7, 8, 1.5],
            "line-opacity": lowResource ? 0.25 : 0.42,
            "line-blur": lowResource ? 0 : 0.35,
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_WEATHER_COVERAGE_FILL_LAYER_ID,
          type: "fill",
          source: PLUGIN_WEATHER_COVERAGE_SOURCE_ID,
          filter: [
            "all",
            ["!=", ["get", "condition"], "clear"],
            ["!=", ["get", "condition"], "unknown"],
          ],
          layout: {
            visibility:
              weatherCoverageVisible && pluginWeatherVisible
                ? "visible"
                : "none",
          },
          paint: {
            "fill-color": weatherCoverageColor("condition"),
            "fill-opacity": lowResource ? 0.07 : 0.12,
            "fill-antialias": true,
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_WEATHER_COVERAGE_PATTERN_LAYER_ID,
          type: "fill",
          source: PLUGIN_WEATHER_COVERAGE_SOURCE_ID,
          filter: [
            "all",
            ["!=", ["get", "condition"], "clear"],
            ["!=", ["get", "condition"], "unknown"],
          ],
          layout: {
            visibility:
              weatherCoverageVisible && pluginWeatherVisible
                ? "visible"
                : "none",
          },
          paint: {
            "fill-pattern": weatherZonePatternExpression("condition", "fill"),
            "fill-opacity": lowResource ? 0.38 : 0.62,
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_WEATHER_COVERAGE_LINE_LAYER_ID,
          type: "line",
          source: PLUGIN_WEATHER_COVERAGE_SOURCE_ID,
          filter: [
            "all",
            ["!=", ["get", "condition"], "clear"],
            ["!=", ["get", "condition"], "unknown"],
          ],
          layout: {
            visibility:
              weatherCoverageVisible && pluginWeatherVisible
                ? "visible"
                : "none",
          },
          paint: {
            "line-color": weatherCoverageColor("condition"),
            "line-width": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              0.55,
              7,
              1.15,
            ],
            "line-opacity": lowResource ? 0.22 : 0.42,
            "line-dasharray": [2, 2],
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_WEATHER_ATMOSPHERE_LAYER_ID,
          type: "circle",
          source: PLUGIN_WEATHER_GRID_SOURCE_ID,
          layout: {
            visibility:
              pluginWeatherVisible &&
              weatherPolicy.atmosphere &&
              mapWeatherEffectsActive
                ? "visible"
                : "none",
          },
          paint: {
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              [
                "+",
                24,
                [
                  "*",
                  2,
                  ["min", 8, ["coalesce", ["get", "precipitation_mm"], 0]],
                ],
              ],
              6,
              [
                "+",
                70,
                [
                  "*",
                  5,
                  ["min", 8, ["coalesce", ["get", "precipitation_mm"], 0]],
                ],
              ],
            ],
            "circle-color": pluginWeatherColor(),
            "circle-opacity": [
              "interpolate",
              ["linear"],
              ["coalesce", ["get", "cloud_cover_percent"], 0],
              0,
              0.04,
              100,
              0.2,
            ],
            "circle-blur": 0.8,
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_WEATHER_CLOUD_HIGHLIGHT_LAYER_ID,
          type: "circle",
          source: PLUGIN_WEATHER_GRID_SOURCE_ID,
          filter: [
            "any",
            [">", ["coalesce", ["get", "cloud_cover_percent"], 0], 20],
            ["==", ["get", "condition"], "cloud"],
            ["==", ["get", "condition"], "rain"],
            ["==", ["get", "condition"], "snow"],
            ["==", ["get", "condition"], "convective"],
          ],
          layout: {
            visibility:
              pluginWeatherVisible &&
              weatherPolicy.clouds &&
              mapWeatherEffectsActive &&
              weatherPolicy.profile === "cinematic"
                ? "visible"
                : "none",
          },
          paint: {
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              12,
              6,
              38,
            ],
            "circle-color": $activeTheme.colors.text,
            "circle-opacity": [
              "interpolate",
              ["linear"],
              ["coalesce", ["get", "cloud_cover_percent"], 50],
              0,
              0.03,
              100,
              0.16,
            ],
            "circle-blur": 0.65,
            "circle-translate": [-5, -5],
            "circle-pitch-alignment": "map",
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_WEATHER_PRECIPITATION_LAYER_ID,
          type: "symbol",
          source: PLUGIN_WEATHER_GRID_SOURCE_ID,
          filter: [
            "any",
            ["==", ["get", "condition"], "rain"],
            ["==", ["get", "condition"], "snow"],
            ["==", ["get", "condition"], "convective"],
          ],
          layout: {
            visibility:
              pluginWeatherVisible &&
              weatherPolicy.precipitation &&
              mapWeatherEffectsActive
                ? "visible"
                : "none",
            "text-field": [
              "case",
              ["==", ["get", "condition"], "snow"],
              "•  ·\n ·  •",
              "╱  ╱\n  ╱  ╱",
            ],
            "text-size": [
              "interpolate",
              ["linear"],
              ["coalesce", ["get", "precipitation_mm"], 0.5],
              0,
              9,
              10,
              18,
            ],
            "text-rotate": ["coalesce", ["get", "wind_direction_degrees"], 0],
            "text-allow-overlap": true,
            "text-ignore-placement": true,
          },
          paint: {
            "text-color": $activeTheme.colors.highlight,
            "text-opacity": 0.76,
            "text-halo-color": $activeTheme.colors.map_halo,
            "text-halo-width": 0.5,
            "text-translate": [0, 0],
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_WEATHER_LIGHTNING_FLASH_LAYER_ID,
          type: "circle",
          source: PLUGIN_WEATHER_GRID_SOURCE_ID,
          filter: ["==", ["get", "condition"], "convective"],
          layout: {
            visibility:
              pluginWeatherVisible &&
              weatherPolicy.lightning &&
              mapWeatherEffectsActive
                ? "visible"
                : "none",
          },
          paint: {
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              32,
              7,
              92,
            ],
            "circle-color": LIGHTNING_COLOR,
            "circle-opacity": 0.035,
            "circle-blur": 0.88,
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_WEATHER_LIGHTNING_LAYER_ID,
          type: "symbol",
          source: PLUGIN_WEATHER_GRID_SOURCE_ID,
          filter: ["==", ["get", "condition"], "convective"],
          layout: {
            visibility:
              pluginWeatherVisible &&
              weatherPolicy.lightning &&
              mapWeatherEffectsActive
                ? "visible"
                : "none",
            "text-field": "ϟ",
            "text-size": ["interpolate", ["linear"], ["zoom"], 1, 12, 7, 26],
            "text-offset": [0.8, -0.8],
            "text-allow-overlap": true,
            "text-ignore-placement": true,
          },
          paint: {
            "text-color": LIGHTNING_COLOR,
            "text-opacity": 0.92,
            "text-halo-color": $activeTheme.colors.map_halo,
            "text-halo-width": 1.2,
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_WEATHER_GRID_LAYER_ID,
          type: "circle",
          source: PLUGIN_WEATHER_GRID_SOURCE_ID,
          layout: { visibility: pluginWeatherVisible ? "visible" : "none" },
          paint: {
            "circle-radius": ["interpolate", ["linear"], ["zoom"], 1, 3, 6, 8],
            "circle-color": pluginWeatherColor(),
            "circle-opacity": 0.8,
            "circle-stroke-color": $activeTheme.colors.map_halo,
            "circle-stroke-width": 1.5,
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_RADAR_COVERAGE_FILL_LAYER_ID,
          type: "fill",
          source: PLUGIN_RADAR_COVERAGE_SOURCE_ID,
          layout: {
            visibility:
              weatherCoverageVisible && pluginWeatherVisible
                ? "visible"
                : "none",
          },
          paint: {
            "fill-color": WEATHER_ZONE_COLORS.radar,
            "fill-opacity": lowResource ? 0.025 : 0.045,
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_RADAR_COVERAGE_PATTERN_LAYER_ID,
          type: "fill",
          source: PLUGIN_RADAR_COVERAGE_SOURCE_ID,
          layout: {
            visibility:
              weatherCoverageVisible && pluginWeatherVisible
                ? "visible"
                : "none",
          },
          paint: {
            "fill-pattern": weatherZonePatternId("radar", "fill"),
            "fill-opacity": lowResource ? 0.3 : 0.5,
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_RADAR_COVERAGE_LINE_LAYER_ID,
          type: "line",
          source: PLUGIN_RADAR_COVERAGE_SOURCE_ID,
          layout: {
            visibility:
              weatherCoverageVisible && pluginWeatherVisible
                ? "visible"
                : "none",
          },
          paint: {
            "line-color": WEATHER_ZONE_COLORS.radar,
            "line-width": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              0.65,
              7,
              1.4,
            ],
            "line-opacity": lowResource ? 0.28 : 0.52,
            "line-dasharray": [3, 2],
          },
        });
        atlasMap.addLayer({
          id: REGION_FILL_LAYER_ID,
          type: "fill",
          source: REGION_SOURCE_ID,
          minzoom: 1.75,
          layout: { visibility: regionsVisible ? "visible" : "none" },
          paint: {
            "fill-color": $activeTheme.colors.highlight,
            "fill-opacity": regionFillOpacity(),
            "fill-opacity-transition": {
              duration: regionMotionDuration(),
            },
          },
        });
        atlasMap.addLayer({
          id: REGION_BOUNDARY_LAYER_ID,
          type: "line",
          source: REGION_SOURCE_ID,
          minzoom: 1.75,
          layout: { visibility: regionsVisible ? "visible" : "none" },
          paint: {
            "line-color": $activeTheme.colors.map_label,
            "line-width": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1.75,
              0.35,
              8,
              1.15,
            ],
            "line-opacity": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1.75,
              0.14,
              4,
              0.3,
              8,
              0.42,
            ],
          },
        });
        atlasMap.addLayer({
          id: REGION_HALO_LAYER_ID,
          type: "line",
          source: REGION_SOURCE_ID,
          minzoom: 1.75,
          layout: { visibility: regionsVisible ? "visible" : "none" },
          paint: {
            "line-color": $activeTheme.colors.highlight,
            "line-width": regionHaloWidth(),
            "line-blur": lowResource ? 0.5 : 2,
            "line-opacity": [
              "case",
              regionHovered,
              lowResource ? 0.7 : 0.9,
              regionSelected,
              0.82,
              0,
            ],
            "line-width-transition": {
              duration: regionMotionDuration(),
            },
            "line-opacity-transition": {
              duration: regionMotionDuration(),
            },
          },
        });
        for (const band of ADMINISTRATIVE_REGION_LABEL_BANDS) {
          atlasMap.addLayer({
            id: `${REGION_LABEL_LAYER_PREFIX}-${band.id}`,
            type: "symbol",
            source: REGION_SOURCE_ID,
            minzoom: band.min_zoom,
            maxzoom: band.max_zoom,
            filter: [
              "<=",
              ["coalesce", ["get", "label_min_zoom"], 99],
              band.maximum_source_min_zoom,
            ],
            layout: {
              visibility: regionsVisible ? "visible" : "none",
              "text-field": ["get", "name"],
              "text-size": [
                "interpolate",
                ["linear"],
                ["zoom"],
                2.5,
                8,
                10,
                11,
              ],
              "text-font": ["Noto Sans Regular"],
              "text-max-width": 9,
              "text-padding": band.text_padding,
              "text-allow-overlap": false,
              "symbol-sort-key": ["coalesce", ["get", "label_rank"], 99],
            },
            paint: {
              "text-color": $activeTheme.colors.map_label,
              "text-opacity": 0.82,
              "text-halo-color": $activeTheme.colors.map_halo,
              "text-halo-width": 1.4,
              "text-opacity-transition": {
                duration: regionMotionDuration(),
              },
            },
          });
        }
        atlasMap.addLayer({
          id: WEATHER_COVERAGE_OUTER_LAYER_ID,
          type: "circle",
          source: WEATHER_SOURCE_ID,
          filter: ["!=", ["get", "effect"], "none"],
          layout: {
            visibility:
              weatherCoverageVisible && weatherVisible ? "visible" : "none",
          },
          paint: {
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              ["+", 30, ["*", 14, ["get", "intensity"]]],
              8,
              ["+", 92, ["*", 42, ["get", "intensity"]]],
            ],
            "circle-color": weatherCoverageColor("effect"),
            "circle-opacity": lowResource ? 0.045 : 0.075,
            "circle-blur": 0.24,
            "circle-pitch-alignment": "map",
          },
        });
        atlasMap.addLayer({
          id: WEATHER_COVERAGE_INNER_LAYER_ID,
          type: "circle",
          source: WEATHER_SOURCE_ID,
          filter: ["!=", ["get", "effect"], "none"],
          layout: {
            visibility:
              weatherCoverageVisible && weatherVisible ? "visible" : "none",
          },
          paint: {
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              ["+", 20, ["*", 10, ["get", "intensity"]]],
              8,
              ["+", 66, ["*", 30, ["get", "intensity"]]],
            ],
            "circle-color": weatherCoverageColor("effect"),
            "circle-opacity": lowResource ? 0.065 : 0.11,
            "circle-blur": 0.18,
            "circle-stroke-color": weatherCoverageColor("effect"),
            "circle-stroke-width": lowResource ? 0.65 : 1.15,
            "circle-stroke-opacity": lowResource ? 0.28 : 0.5,
            "circle-pitch-alignment": "map",
          },
        });
        atlasMap.addLayer({
          id: WEATHER_COVERAGE_PATTERN_LAYER_ID,
          type: "symbol",
          source: WEATHER_SOURCE_ID,
          filter: ["!=", ["get", "effect"], "none"],
          layout: {
            visibility:
              weatherCoverageVisible && weatherVisible ? "visible" : "none",
            "icon-image": weatherZonePatternExpression("effect", "marker"),
            "icon-size": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              ["+", 0.65, ["*", 0.25, ["get", "intensity"]]],
              8,
              ["+", 1.75, ["*", 0.85, ["get", "intensity"]]],
            ],
            "icon-allow-overlap": true,
            "icon-ignore-placement": true,
            "icon-pitch-alignment": "map",
            "icon-rotation-alignment": "map",
          },
          paint: {
            "icon-opacity": lowResource ? 0.38 : 0.58,
          },
        });
        atlasMap.addLayer({
          id: WEATHER_ATMOSPHERE_LAYER_ID,
          type: "heatmap",
          source: WEATHER_SOURCE_ID,
          filter: ["==", ["get", "has_metar"], true],
          layout: {
            visibility:
              weatherVisible &&
              weatherPolicy.atmosphere &&
              mapWeatherEffectsActive
                ? "visible"
                : "none",
          },
          paint: {
            "heatmap-weight": [
              "interpolate",
              ["linear"],
              ["get", "severity"],
              0,
              0.08,
              1,
              1,
            ],
            "heatmap-intensity": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              0.7,
              8,
              1.25,
            ],
            "heatmap-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              28,
              8,
              78,
            ],
            "heatmap-opacity": 0.68,
            "heatmap-color": [
              "interpolate",
              ["linear"],
              ["heatmap-density"],
              0,
              "rgba(0, 0, 0, 0)",
              0.15,
              $activeTheme.colors.success,
              0.42,
              $activeTheme.colors.highlight,
              0.72,
              $activeTheme.colors.accent,
              1,
              $activeTheme.colors.danger,
            ],
          },
        });
        atlasMap.addLayer({
          id: WEATHER_CLOUD_SHADOW_LAYER_ID,
          type: "circle",
          source: WEATHER_SOURCE_ID,
          filter: [
            "any",
            ["==", ["get", "effect"], "rain"],
            ["==", ["get", "effect"], "snow"],
            ["==", ["get", "effect"], "convective"],
          ],
          layout: {
            visibility:
              weatherVisible && weatherPolicy.clouds && mapWeatherEffectsActive
                ? "visible"
                : "none",
          },
          paint: {
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              ["+", 16, ["*", 8, ["get", "intensity"]]],
              8,
              ["+", 44, ["*", 28, ["get", "intensity"]]],
            ],
            "circle-color": $activeTheme.colors.map_halo,
            "circle-opacity": 0.34,
            "circle-blur": 0.72,
            "circle-translate": [6, 8],
            "circle-pitch-alignment": "map",
          },
        });
        atlasMap.addLayer({
          id: WEATHER_CLOUD_BODY_LAYER_ID,
          type: "circle",
          source: WEATHER_SOURCE_ID,
          filter: [
            "any",
            ["==", ["get", "effect"], "rain"],
            ["==", ["get", "effect"], "snow"],
            ["==", ["get", "effect"], "convective"],
          ],
          layout: {
            visibility:
              weatherVisible && weatherPolicy.clouds && mapWeatherEffectsActive
                ? "visible"
                : "none",
          },
          paint: {
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              ["+", 13, ["*", 7, ["get", "intensity"]]],
              8,
              ["+", 36, ["*", 24, ["get", "intensity"]]],
            ],
            "circle-color": $activeTheme.colors.text_muted,
            "circle-opacity": [
              "interpolate",
              ["linear"],
              ["get", "intensity"],
              0,
              0.12,
              1,
              0.4,
            ],
            "circle-blur": 0.48,
            "circle-stroke-color": $activeTheme.colors.map_label,
            "circle-stroke-width": 1,
            "circle-stroke-opacity": 0.22,
            "circle-pitch-alignment": "map",
          },
        });
        atlasMap.addLayer({
          id: WEATHER_CLOUD_HIGHLIGHT_LAYER_ID,
          type: "circle",
          source: WEATHER_SOURCE_ID,
          filter: [
            "any",
            ["==", ["get", "effect"], "rain"],
            ["==", ["get", "effect"], "snow"],
            ["==", ["get", "effect"], "convective"],
          ],
          layout: {
            visibility:
              weatherVisible &&
              weatherPolicy.clouds &&
              mapWeatherEffectsActive &&
              weatherPolicy.profile === "cinematic"
                ? "visible"
                : "none",
          },
          paint: {
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              ["+", 7, ["*", 4, ["get", "intensity"]]],
              8,
              ["+", 20, ["*", 13, ["get", "intensity"]]],
            ],
            "circle-color": $activeTheme.colors.text,
            "circle-opacity": 0.18,
            "circle-blur": 0.58,
            "circle-translate": [-5, -5],
            "circle-pitch-alignment": "map",
          },
        });
        atlasMap.addLayer({
          id: WEATHER_EFFECT_LAYER_ID,
          type: "circle",
          source: WEATHER_SOURCE_ID,
          filter: ["!=", ["get", "effect"], "none"],
          layout: {
            visibility:
              weatherVisible &&
              weatherPolicy.atmosphere &&
              mapWeatherEffectsActive
                ? "visible"
                : "none",
          },
          paint: {
            "circle-radius": weatherEffectRadius(0.35),
            "circle-color": [
              "match",
              ["get", "effect"],
              "rain",
              $activeTheme.colors.highlight,
              "snow",
              $activeTheme.colors.map_label,
              "convective",
              $activeTheme.colors.danger,
              "obscuration",
              $activeTheme.colors.text_muted,
              "dust",
              DUST_OUTER_COLOR,
              $activeTheme.colors.accent,
            ],
            "circle-opacity": 0.26,
            "circle-blur": 0.5,
            "circle-stroke-color": $activeTheme.colors.highlight,
            "circle-stroke-width": 1.5,
            "circle-stroke-opacity": 0.72,
          },
        });
        atlasMap.addLayer({
          id: WEATHER_DUST_LAYER_ID,
          type: "circle",
          source: WEATHER_SOURCE_ID,
          filter: ["==", ["get", "effect"], "dust"],
          layout: {
            visibility:
              weatherVisible && weatherPolicy.dust && mapWeatherEffectsActive
                ? "visible"
                : "none",
          },
          paint: {
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              ["+", 24, ["*", 10, ["get", "intensity"]]],
              8,
              ["+", 62, ["*", 34, ["get", "intensity"]]],
            ],
            "circle-color": DUST_OUTER_COLOR,
            "circle-opacity": 0.32,
            "circle-blur": 0.78,
            "circle-translate": [0, 0],
            "circle-pitch-alignment": "map",
          },
        });
        atlasMap.addLayer({
          id: WEATHER_DUST_CORE_LAYER_ID,
          type: "circle",
          source: WEATHER_SOURCE_ID,
          filter: ["==", ["get", "effect"], "dust"],
          layout: {
            visibility:
              weatherVisible &&
              weatherPolicy.dust &&
              mapWeatherEffectsActive &&
              weatherPolicy.profile === "cinematic"
                ? "visible"
                : "none",
          },
          paint: {
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              12,
              8,
              38,
            ],
            "circle-color": DUST_CORE_COLOR,
            "circle-opacity": 0.24,
            "circle-blur": 0.52,
            "circle-translate": [-4, -2],
            "circle-pitch-alignment": "map",
          },
        });
        atlasMap.addLayer({
          id: WEATHER_PRECIPITATION_LAYER_ID,
          type: "symbol",
          source: WEATHER_SOURCE_ID,
          filter: [
            "any",
            ["==", ["get", "effect"], "rain"],
            ["==", ["get", "effect"], "snow"],
            ["==", ["get", "effect"], "convective"],
          ],
          layout: {
            visibility:
              weatherVisible &&
              weatherPolicy.precipitation &&
              mapWeatherEffectsActive
                ? "visible"
                : "none",
            "text-field": [
              "case",
              ["==", ["get", "effect"], "snow"],
              "• · •\n · • ·",
              "╱ ╱ ╱\n ╱ ╱ ╱",
            ],
            "text-size": [
              "interpolate",
              ["linear"],
              ["get", "intensity"],
              0,
              10,
              1,
              20,
            ],
            "text-rotate": ["get", "wind_bearing"],
            "text-allow-overlap": true,
            "text-ignore-placement": true,
          },
          paint: {
            "text-color": $activeTheme.colors.highlight,
            "text-opacity": 0.82,
            "text-halo-color": $activeTheme.colors.map_halo,
            "text-halo-width": 0.6,
            "text-translate": [0, 0],
          },
        });
        atlasMap.addLayer({
          id: WEATHER_LIGHTNING_FLASH_LAYER_ID,
          type: "circle",
          source: WEATHER_SOURCE_ID,
          filter: ["==", ["get", "effect"], "convective"],
          layout: {
            visibility:
              weatherVisible &&
              weatherPolicy.lightning &&
              mapWeatherEffectsActive
                ? "visible"
                : "none",
          },
          paint: {
            "circle-radius": [
              "interpolate",
              ["linear"],
              ["zoom"],
              1,
              40,
              8,
              116,
            ],
            "circle-color": LIGHTNING_COLOR,
            "circle-opacity": 0.035,
            "circle-blur": 0.9,
          },
        });
        atlasMap.addLayer({
          id: WEATHER_LIGHTNING_LAYER_ID,
          type: "symbol",
          source: WEATHER_SOURCE_ID,
          filter: ["==", ["get", "effect"], "convective"],
          layout: {
            visibility:
              weatherVisible &&
              weatherPolicy.lightning &&
              mapWeatherEffectsActive
                ? "visible"
                : "none",
            "text-field": "ϟ",
            "text-size": ["interpolate", ["linear"], ["zoom"], 1, 14, 8, 32],
            "text-offset": [0.9, -0.8],
            "text-allow-overlap": true,
            "text-ignore-placement": true,
          },
          paint: {
            "text-color": LIGHTNING_COLOR,
            "text-opacity": 0.94,
            "text-halo-color": $activeTheme.colors.map_halo,
            "text-halo-width": 1.4,
          },
        });
        atlasMap.addLayer({
          id: WEATHER_WIND_LAYER_ID,
          type: "line",
          source: WEATHER_WIND_SOURCE_ID,
          filter: ["==", ["get", "feature_type"], "wind_path"],
          layout: {
            visibility:
              weatherVisible && weatherPolicy.atmosphere ? "visible" : "none",
          },
          paint: {
            "line-color": [
              "interpolate",
              ["linear"],
              ["get", "wind_speed_kt"],
              0,
              $activeTheme.colors.success,
              20,
              $activeTheme.colors.highlight,
              40,
              $activeTheme.colors.accent,
              60,
              $activeTheme.colors.danger,
            ],
            "line-width": [
              "interpolate",
              ["linear"],
              ["get", "wind_speed_kt"],
              1,
              1.5,
              50,
              4,
            ],
            "line-opacity": 0.82,
            "line-blur": 0.5,
            "line-dasharray": [1.4, 1],
          },
        });
        atlasMap.addLayer({
          id: WEATHER_WIND_TIP_LAYER_ID,
          type: "symbol",
          source: WEATHER_WIND_SOURCE_ID,
          filter: ["==", ["get", "feature_type"], "wind_tip"],
          layout: {
            visibility:
              weatherVisible && weatherPolicy.atmosphere ? "visible" : "none",
            "text-field": "▲",
            "text-size": 11,
            "text-rotate": ["get", "bearing"],
            "text-rotation-alignment": "map",
            "text-pitch-alignment": "map",
            "text-allow-overlap": true,
          },
          paint: {
            "text-color": $activeTheme.colors.highlight,
            "text-halo-color": $activeTheme.colors.map_halo,
            "text-halo-width": 1.25,
          },
        });
        atlasMap.addLayer({
          id: WEATHER_LAYER_ID,
          type: "circle",
          source: WEATHER_SOURCE_ID,
          layout: { visibility: weatherVisible ? "visible" : "none" },
          paint: {
            "circle-radius": ["interpolate", ["linear"], ["zoom"], 1, 8, 8, 16],
            "circle-color": $activeTheme.colors.text_muted,
            "circle-opacity": 0.32,
            "circle-stroke-color": $activeTheme.colors.map_halo,
            "circle-stroke-width": 2,
            "circle-stroke-opacity": 0.9,
          },
        });
        atlasMap.addLayer({
          id: WEATHER_LABEL_LAYER_ID,
          type: "symbol",
          source: WEATHER_SOURCE_ID,
          minzoom: 3,
          layout: {
            visibility: weatherVisible ? "visible" : "none",
            "text-field": [
              "concat",
              ["get", "station_icao"],
              " · ",
              ["upcase", ["get", "category"]],
            ],
            "text-size": 10,
            "text-offset": [0, -1.8],
            "text-anchor": "bottom",
            "text-allow-overlap": false,
          },
          paint: {
            "text-color": $activeTheme.colors.map_label,
            "text-halo-color": $activeTheme.colors.map_halo,
            "text-halo-width": 1.5,
          },
        });
        atlasMap.addLayer({
          id: PLANNED_ROUTE_LAYER_ID,
          type: "line",
          source: ROUTE_SOURCE_ID,
          filter: ["==", ["get", "kind"], "planned"],
          paint: {
            "line-color": $activeTheme.colors.highlight,
            "line-width": ["interpolate", ["linear"], ["zoom"], 1, 2, 8, 5],
            "line-dasharray": [2, 2],
            "line-opacity": 0.9,
          },
        });
        atlasMap.addLayer({
          id: RECORDED_ROUTE_LAYER_ID,
          type: "line",
          source: ROUTE_SOURCE_ID,
          filter: ["==", ["get", "kind"], "recorded"],
          paint: {
            "line-color": $activeTheme.colors.accent,
            "line-width": ["interpolate", ["linear"], ["zoom"], 1, 2.5, 8, 6],
            "line-opacity": 0.92,
          },
        });
        atlasMap.addLayer({
          id: ROUTE_MARKER_LAYER_ID,
          type: "circle",
          source: ROUTE_MARKER_SOURCE_ID,
          paint: {
            "circle-radius": ["interpolate", ["linear"], ["zoom"], 1, 3, 8, 6],
            "circle-color": $activeTheme.colors.highlight,
            "circle-stroke-color": $activeTheme.colors.map_halo,
            "circle-stroke-width": 2,
          },
        });
        atlasMap.addLayer({
          id: ROUTE_LABEL_LAYER_ID,
          type: "symbol",
          source: ROUTE_MARKER_SOURCE_ID,
          minzoom: 3,
          layout: {
            "text-field": ["get", "label"],
            "text-size": 10,
            "text-offset": [0, 1.25],
            "text-anchor": "top",
            "text-allow-overlap": false,
          },
          paint: {
            "text-color": $activeTheme.colors.highlight,
            "text-halo-color": $activeTheme.colors.map_halo,
            "text-halo-width": 1.5,
          },
        });
        atlasMap.addSource(ROUTE_WEATHER_SOURCE_ID, {
          type: "geojson",
          data: routeWeatherLineFeatures(routeWeather),
        });
        atlasMap.addLayer({
          id: ROUTE_WEATHER_HALO_LAYER_ID,
          type: "line",
          source: ROUTE_WEATHER_SOURCE_ID,
          layout: {
            visibility:
              routeVisible && pluginWeatherVisible && routeWeather
                ? "visible"
                : "none",
          },
          paint: {
            "line-color": "#101923",
            "line-width": ["interpolate", ["linear"], ["zoom"], 1, 7, 8, 16],
            "line-opacity": 0.5,
          },
        });
        atlasMap.addLayer({
          id: ROUTE_WEATHER_SUPPORTED_LAYER_ID,
          type: "line",
          source: ROUTE_WEATHER_SOURCE_ID,
          filter: ["==", ["get", "temporal_support"], "eta_matched"],
          layout: {
            visibility:
              routeVisible && pluginWeatherVisible && routeWeather
                ? "visible"
                : "none",
          },
          paint: {
            "line-color": weatherCoverageColor("condition"),
            "line-width": ["interpolate", ["linear"], ["zoom"], 1, 4.5, 8, 10],
            "line-opacity": 0.82,
          },
        });
        atlasMap.addLayer({
          id: ROUTE_WEATHER_CURRENT_CONTEXT_LAYER_ID,
          type: "line",
          source: ROUTE_WEATHER_SOURCE_ID,
          filter: ["==", ["get", "temporal_support"], "current_context"],
          layout: {
            visibility:
              routeVisible && pluginWeatherVisible && routeWeather
                ? "visible"
                : "none",
          },
          paint: {
            "line-color": weatherCoverageColor("condition"),
            "line-width": ["interpolate", ["linear"], ["zoom"], 1, 4, 8, 9],
            "line-opacity": 0.58,
            "line-dasharray": [2.2, 1.6],
          },
        });
        atlasMap.addLayer({
          id: ROUTE_WEATHER_UNAVAILABLE_LAYER_ID,
          type: "line",
          source: ROUTE_WEATHER_SOURCE_ID,
          filter: ["==", ["get", "support"], "unavailable"],
          layout: {
            visibility:
              routeVisible && pluginWeatherVisible && routeWeather
                ? "visible"
                : "none",
          },
          paint: {
            "line-color": "#a9b3bd",
            "line-width": ["interpolate", ["linear"], ["zoom"], 1, 4, 8, 9],
            "line-opacity": 0.72,
            "line-dasharray": [1.2, 1.8],
          },
        });
        atlasMap.addSource(DISPATCH_ROUTE_SOURCE_ID, {
          type: "geojson",
          data: atlasRouteGeoJson(route),
        });
        atlasMap.addLayer({
          id: DISPATCH_ROUTE_LINE_LAYER_ID,
          type: "line",
          source: DISPATCH_ROUTE_SOURCE_ID,
          filter: ["==", ["get", "feature_type"], "route"],
          paint: {
            "line-color": $activeTheme.colors.highlight,
            "line-width": ["interpolate", ["linear"], ["zoom"], 1, 1.5, 8, 4],
            "line-opacity": 0.88,
            "line-dasharray": [2, 1.4],
          },
        });
        atlasMap.addLayer({
          id: FBO_LAYER_ID,
          type: "circle",
          source: FBO_SOURCE_ID,
          paint: {
            "circle-radius": ["interpolate", ["linear"], ["zoom"], 1, 6, 7, 10],
            "circle-color": $activeTheme.colors.map_fbo,
            "circle-stroke-color": $activeTheme.colors.map_halo,
            "circle-stroke-width": 2.5,
            "circle-opacity": 0.95,
          },
        });
        atlasMap.addLayer({
          id: FBO_LABEL_LAYER_ID,
          type: "symbol",
          source: FBO_SOURCE_ID,
          minzoom: 4.5,
          layout: {
            "text-field": ["coalesce", ["get", "name"], ["get", "icao"], "FBO"],
            "text-size": 11,
            "text-offset": [0, 1.5],
            "text-anchor": "top",
          },
          paint: {
            "text-color": $activeTheme.colors.map_label,
            "text-halo-color": $activeTheme.colors.map_halo,
            "text-halo-width": 1.5,
          },
        });
        atlasMap.addLayer({
          id: FLEET_LAYER_ID,
          type: "circle",
          source: FLEET_SOURCE_ID,
          paint: {
            "circle-radius": ["interpolate", ["linear"], ["zoom"], 1, 5, 7, 9],
            "circle-color": $activeTheme.colors.map_aircraft,
            "circle-stroke-color": $activeTheme.colors.map_halo,
            "circle-stroke-width": 2,
            "circle-opacity": 0.92,
          },
        });
        atlasMap.addLayer({
          id: FLEET_LABEL_LAYER_ID,
          type: "symbol",
          source: FLEET_SOURCE_ID,
          minzoom: 4.75,
          layout: {
            "text-field": [
              "coalesce",
              ["get", "registration"],
              ["get", "model"],
              "Aircraft",
            ],
            "text-size": 11,
            "text-offset": [0, 1.35],
            "text-anchor": "top",
            "text-allow-overlap": false,
          },
          paint: {
            "text-color": $activeTheme.colors.map_label,
            "text-halo-color": $activeTheme.colors.map_halo,
            "text-halo-width": 1.5,
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_LAYER_ID,
          type: "circle",
          source: PLUGIN_SOURCE_ID,
          paint: {
            "circle-radius": ["interpolate", ["linear"], ["zoom"], 1, 8, 7, 14],
            "circle-color": $activeTheme.colors.highlight,
            "circle-opacity": 0.16,
            "circle-stroke-color": $activeTheme.colors.highlight,
            "circle-stroke-width": 2,
            "circle-stroke-opacity": 0.95,
          },
        });
        atlasMap.addLayer({
          id: PLUGIN_LABEL_LAYER_ID,
          type: "symbol",
          source: PLUGIN_SOURCE_ID,
          minzoom: 4,
          layout: {
            "text-field": ["get", "label"],
            "text-size": 10,
            "text-offset": [0, -1.6],
            "text-anchor": "bottom",
            "text-allow-overlap": false,
          },
          paint: {
            "text-color": $activeTheme.colors.highlight,
            "text-halo-color": $activeTheme.colors.map_halo,
            "text-halo-width": 1.5,
          },
        });
        atlasMap.addLayer({
          id: DISPATCH_ROUTE_POINT_LAYER_ID,
          type: "circle",
          source: DISPATCH_ROUTE_SOURCE_ID,
          filter: ["==", ["get", "feature_type"], "point"],
          paint: {
            "circle-radius": ["interpolate", ["linear"], ["zoom"], 1, 5, 8, 9],
            "circle-color": $activeTheme.colors.highlight,
            "circle-stroke-color": $activeTheme.colors.map_halo,
            "circle-stroke-width": 2,
            "circle-opacity": 0.96,
          },
        });
        atlasMap.addLayer({
          id: DISPATCH_ROUTE_LABEL_LAYER_ID,
          type: "symbol",
          source: DISPATCH_ROUTE_SOURCE_ID,
          filter: ["==", ["get", "feature_type"], "point"],
          minzoom: 3,
          layout: {
            "text-field": ["get", "ident"],
            "text-size": 10,
            "text-offset": [0, -1.5],
            "text-anchor": "bottom",
            "text-allow-overlap": false,
          },
          paint: {
            "text-color": $activeTheme.colors.highlight,
            "text-halo-color": $activeTheme.colors.map_halo,
            "text-halo-width": 1.5,
          },
        });
        atlasMap.on("click", FLEET_LAYER_ID, (event) => {
          const aircraftId = event.features?.[0]?.properties?.id;
          if (typeof aircraftId === "string") onselectaircraft(aircraftId);
        });
        atlasMap.on("click", FBO_LAYER_ID, (event) => {
          const fboId = event.features?.[0]?.properties?.id;
          if (typeof fboId === "string") onselectfbo(fboId);
        });
        atlasMap.on("click", ROUTE_MARKER_LAYER_ID, (event) => {
          const pointId = event.features?.[0]?.properties?.id;
          if (typeof pointId === "string") onselectroutepoint(pointId);
        });
        atlasMap.on("click", WEATHER_LAYER_ID, (event) => {
          const stationId = event.features?.[0]?.properties?.id;
          if (typeof stationId === "string") onselectweatherstation(stationId);
        });
        atlasMap.on("click", REGION_FILL_LAYER_ID, (event) => {
          const foregroundFeatures = atlasMap.queryRenderedFeatures(
            event.point,
            {
              layers: [
                FLEET_LAYER_ID,
                FBO_LAYER_ID,
                ROUTE_MARKER_LAYER_ID,
                DISPATCH_ROUTE_POINT_LAYER_ID,
                WEATHER_LAYER_ID,
                PLUGIN_LAYER_ID,
              ],
            },
          );
          if (foregroundFeatures.length > 0) return;
          const region = administrativeRegionFromMapFeature(
            event.features?.[0],
          );
          if (region) onselectregion(region);
        });
        atlasMap.on("mousemove", REGION_FILL_LAYER_ID, (event) => {
          const region = administrativeRegionFromMapFeature(
            event.features?.[0],
          );
          if (!region || region.feature_id === hoveredRegionFeatureId) return;
          clearHoveredRegion();
          hoveredRegionFeatureId = region.feature_id;
          atlasMap.setFeatureState(
            { source: REGION_SOURCE_ID, id: region.feature_id },
            { hover: true },
          );
          atlasMap.getCanvas().style.cursor = "pointer";
          onhoverregion(region);
        });
        atlasMap.on("mouseleave", REGION_FILL_LAYER_ID, () => {
          clearHoveredRegion();
          atlasMap.getCanvas().style.cursor = "";
        });
        atlasMap.on("click", DISPATCH_ROUTE_POINT_LAYER_ID, (event) => {
          const featureId = event.features?.[0]?.properties?.id;
          if (typeof featureId === "string") onselectroutefeature(featureId);
        });
        atlasMap.on("mouseenter", FLEET_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "pointer";
        });
        atlasMap.on("mouseleave", FLEET_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "";
        });
        atlasMap.on("mouseenter", FBO_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "pointer";
        });
        atlasMap.on("mouseleave", FBO_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "";
        });
        atlasMap.on("mouseenter", ROUTE_MARKER_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "pointer";
        });
        atlasMap.on("mouseleave", ROUTE_MARKER_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "";
        });
        atlasMap.on("mouseenter", WEATHER_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "pointer";
        });
        atlasMap.on("mouseleave", WEATHER_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "";
        });
        atlasMap.on("mouseenter", DISPATCH_ROUTE_POINT_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "pointer";
        });
        atlasMap.on("mouseleave", DISPATCH_ROUTE_POINT_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "";
        });
        mapReady = true;
        updateAtlas();
      });
    });

    return () => {
      cancelled = true;
      window.clearInterval(daylightRefreshInterval);
      window.clearInterval(radarAnimationInterval);
      motionQuery.removeEventListener("change", updateMotionPreference);
      stopWeatherAnimation();
      disposeWeatherRenderer();
      weatherRendererStatus = { state: "disabled" };
      clearHoveredRegion();
      map?.remove();
    };
  });
</script>

<div
  bind:this={mapContainer}
  class="map"
  aria-label={$translation("atlas-map-label")}
></div>
<canvas
  bind:this={weatherCanvas}
  class="weather-render-canvas"
  class:visible={threeWeatherActive}
  aria-hidden="true"
></canvas>

{#if weatherVisible && plottedWeatherStationCount > 0}
  <div class="weather-render-status" role="status">
    <span>{weatherStatusPresentation.title}</span>
    <strong>{weatherStatusPresentation.stationCount}</strong>
    <small>{weatherStatusPresentation.detail}</small>
    <em>{weatherStatusPresentation.sourceBoundary}</em>
  </div>
{/if}

{#if pluginWeatherVisible && primaryRadarFrame && primaryRadarTimeline}
  <section
    class="radar-timeline"
    aria-label={$translation("atlas-radar-timeline-title")}
  >
    <span
      >{$translation("atlas-radar-timeline-title")} · {primaryRadarTimeline.provider}</span
    >
    <strong>{formatRadarFrameTime(primaryRadarFrame.frame_time)}</strong>
    <small>
      {$translation("atlas-radar-timeline-position", {
        current: primaryRadarFramePosition,
        total: primaryRadarTimeline.frames.length,
      })}
    </small>
    <div class="radar-timeline-controls">
      <button
        type="button"
        disabled={radarStaticPresentation || primaryRadarFramePosition <= 1}
        aria-label={$translation("atlas-radar-previous-frame")}
        onclick={selectPreviousRadarFrame}>◀</button
      >
      <button
        type="button"
        disabled={radarStaticPresentation || radarFrameCount <= 1}
        aria-label={$translation(
          radarPlaying ? "atlas-radar-pause" : "atlas-radar-play",
        )}
        onclick={toggleRadarPlayback}>{radarPlaying ? "Ⅱ" : "▶"}</button
      >
      <button
        type="button"
        disabled={radarStaticPresentation ||
          primaryRadarFramePosition >= primaryRadarTimeline.frames.length}
        aria-label={$translation("atlas-radar-next-frame")}
        onclick={selectNextRadarFrame}>▶</button
      >
    </div>
    {#if radarStaticPresentation}
      <em>{$translation("atlas-radar-static-mode")}</em>
    {/if}
    <em
      >{$translation(
        primaryRadarHasCoverageMask
          ? "atlas-radar-no-data-key"
          : "atlas-radar-coverage-unknown",
      )}</em
    >
  </section>
{/if}

<style>
  .weather-render-canvas {
    pointer-events: none;
    position: absolute;
    z-index: 1;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
  }

  .weather-render-canvas.visible {
    opacity: 1;
  }

  .weather-render-status {
    pointer-events: none;
    position: absolute;
    z-index: 3;
    right: 22px;
    bottom: 22px;
    width: max-content;
    max-width: min(430px, calc(100% - 40px));
    border: 1px solid var(--color-highlight-border);
    border-radius: 4px;
    padding: 9px 13px;
    color: var(--color-text);
    background: var(--color-surface-translucent);
    box-shadow: 0 12px 28px var(--color-shadow);
    text-align: center;
    backdrop-filter: blur(9px);
  }

  .weather-render-status span,
  .weather-render-status small,
  .weather-render-status em {
    display: block;
  }

  .radar-timeline {
    position: absolute;
    z-index: 3;
    left: 22px;
    bottom: 22px;
    display: grid;
    gap: 3px;
    min-width: 220px;
    border: 1px solid var(--color-highlight-border);
    border-radius: 4px;
    padding: 9px 13px;
    color: var(--color-text);
    background: var(--color-surface-translucent);
    box-shadow: 0 12px 28px var(--color-shadow);
    backdrop-filter: blur(9px);
  }

  .radar-timeline > span {
    color: var(--color-highlight);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.11em;
    text-transform: uppercase;
  }

  .radar-timeline > strong {
    font-family: Georgia, serif;
    font-size: 15px;
    font-weight: 400;
  }

  .radar-timeline > small,
  .radar-timeline > em {
    color: var(--color-text-muted);
    font-size: 9px;
    font-style: normal;
    line-height: 1.45;
  }

  .radar-timeline > em {
    color: var(--color-highlight);
  }

  .radar-timeline-controls {
    display: flex;
    gap: 5px;
    margin: 3px 0;
  }

  .radar-timeline-controls button {
    min-width: 32px;
    border: 1px solid var(--color-line);
    border-radius: 3px;
    padding: 3px 7px;
    color: var(--color-text);
    background: var(--color-surface-soft);
    cursor: pointer;
  }

  .radar-timeline-controls button:disabled {
    cursor: default;
    opacity: 0.4;
  }

  .weather-render-status span {
    color: var(--color-highlight);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.11em;
    text-transform: uppercase;
  }

  .weather-render-status strong {
    display: block;
    margin: 3px 0;
    font-family: Georgia, serif;
    font-size: 15px;
    font-weight: 400;
  }

  .weather-render-status small,
  .weather-render-status em {
    color: var(--color-text-muted);
    font-size: 9px;
    font-style: normal;
    line-height: 1.45;
  }

  .weather-render-status em {
    margin-top: 2px;
    color: var(--color-highlight);
  }

  @media (max-width: 760px) {
    .weather-render-status {
      top: 12px;
      right: 12px;
      bottom: auto;
      max-width: calc(100% - 24px);
      padding: 7px 10px;
    }

    .radar-timeline {
      left: 12px;
      bottom: 12px;
      min-width: 190px;
      max-width: calc(100% - 24px);
      padding: 7px 10px;
    }
  }
</style>
