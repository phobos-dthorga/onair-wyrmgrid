<script lang="ts">
  import type {
    ExpressionSpecification,
    GeoJSONSource,
    ImageSource,
    Map as MapLibreMap,
  } from "maplibre-gl";
  import "maplibre-gl/dist/maplibre-gl.css";
  import { onMount } from "svelte";
  import type { AtlasRouteView } from "$lib/dispatch/types";
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
  import {
    pluginRadarFrames,
    pluginWeatherGridFeatures,
  } from "$lib/weather/pluginWeather";
  import { ATLAS_HOME_CENTER, balancedOverviewCoordinates } from "./camera";
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
    enhancedWeather,
    regionsVisible,
    lowResource,
    selectedRegionId,
    selectedRoutePointId,
    selectedWeatherStationId,
    route,
    routeVisible,
    selectedAircraftId,
    selectedFboId,
    selectedRouteFeatureId,
    focusRequest,
    onselectaircraft,
    onselectfbo,
    onselectroutepoint,
    onselectweatherstation,
    onselectregion,
    onhoverregion,
    onselectroutefeature,
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
    enhancedWeather: boolean;
    regionsVisible: boolean;
    lowResource: boolean;
    selectedRegionId?: string;
    selectedRoutePointId?: string;
    selectedWeatherStationId?: string;
    route?: AtlasRouteView;
    routeVisible: boolean;
    selectedAircraftId: string | null;
    selectedFboId: string | null;
    selectedRouteFeatureId: string | null;
    focusRequest: AtlasFocusRequest | null;
    onselectaircraft: (aircraftId: string) => void;
    onselectfbo: (fboId: string) => void;
    onselectroutepoint: (pointId: string) => void;
    onselectweatherstation: (stationId: string) => void;
    onselectregion: (region: AtlasAdministrativeRegion) => void;
    onhoverregion: (region?: AtlasAdministrativeRegion) => void;
    onselectroutefeature: (featureId: string) => void;
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
  const PLUGIN_WEATHER_ATMOSPHERE_LAYER_ID =
    "wyrmgrid-plugin-weather-atmosphere";
  const PLUGIN_WEATHER_GRID_LAYER_ID = "wyrmgrid-plugin-weather-grid-points";
  const PLUGIN_RADAR_PREFIX = "wyrmgrid-plugin-radar";
  const ROUTE_SOURCE_ID = "wyrmgrid-flight-routes";
  const ROUTE_MARKER_SOURCE_ID = "wyrmgrid-flight-route-markers";
  const PLANNED_ROUTE_LAYER_ID = "wyrmgrid-planned-flight-route";
  const RECORDED_ROUTE_LAYER_ID = "wyrmgrid-recorded-flight-route";
  const ROUTE_MARKER_LAYER_ID = "wyrmgrid-flight-route-markers";
  const ROUTE_LABEL_LAYER_ID = "wyrmgrid-flight-route-labels";
  const WEATHER_SOURCE_ID = "wyrmgrid-flight-weather";
  const WEATHER_WIND_SOURCE_ID = "wyrmgrid-flight-weather-wind";
  const WEATHER_ATMOSPHERE_LAYER_ID = "wyrmgrid-flight-weather-atmosphere";
  const WEATHER_EFFECT_LAYER_ID = "wyrmgrid-flight-weather-effects";
  const WEATHER_WIND_LAYER_ID = "wyrmgrid-flight-weather-wind-paths";
  const WEATHER_WIND_TIP_LAYER_ID = "wyrmgrid-flight-weather-wind-tips";
  const WEATHER_LAYER_ID = "wyrmgrid-flight-weather-stations";
  const WEATHER_LABEL_LAYER_ID = "wyrmgrid-flight-weather-labels";
  const DISPATCH_ROUTE_SOURCE_ID = "wyrmgrid-dispatch-route";
  const DISPATCH_ROUTE_LINE_LAYER_ID = "wyrmgrid-dispatch-route-line";
  const DISPATCH_ROUTE_POINT_LAYER_ID = "wyrmgrid-dispatch-route-points";
  const DISPATCH_ROUTE_LABEL_LAYER_ID = "wyrmgrid-dispatch-route-labels";

  let mapContainer: HTMLDivElement;
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
    return [
      "interpolate",
      ["linear"],
      ["zoom"],
      1,
      14 + pulse * 6,
      8,
      38 + pulse * 18,
    ];
  }

  function stopWeatherAnimation(): void {
    if (weatherAnimationFrame !== undefined) {
      window.cancelAnimationFrame(weatherAnimationFrame);
      weatherAnimationFrame = undefined;
    }
  }

  function applyStaticWeatherEffect(): void {
    if (!map?.getLayer(WEATHER_EFFECT_LAYER_ID)) return;
    map.setPaintProperty(
      WEATHER_EFFECT_LAYER_ID,
      "circle-radius",
      weatherEffectRadius(0.35),
    );
    map.setPaintProperty(WEATHER_EFFECT_LAYER_ID, "circle-opacity", 0.26);
  }

  function updateWeatherAnimation(hasWeatherEffects: boolean): void {
    if (
      !map ||
      !weatherVisible ||
      !enhancedWeather ||
      lowResource ||
      prefersReducedMotion ||
      !hasWeatherEffects
    ) {
      stopWeatherAnimation();
      applyStaticWeatherEffect();
      return;
    }
    if (weatherAnimationFrame !== undefined) return;

    const animate = (time: number) => {
      weatherAnimationFrame = window.requestAnimationFrame(animate);
      if (!map || time - weatherAnimationTime < 1000 / 24) return;
      weatherAnimationTime = time;
      const pulse = (Math.sin(time / 620) + 1) / 2;
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

  function pluginRadarMapId(frameId: string): string {
    return `${PLUGIN_RADAR_PREFIX}-${frameId.replaceAll(/[^A-Za-z0-9_-]/g, "-")}`;
  }

  function syncPluginRadarFrames(): void {
    if (!map || !mapReady) return;
    const frames = pluginRadarFrames(pluginWeatherLayers);
    const nextLayerIds = new Set<string>();
    const nextFrameVersions = new Map<string, string>();
    for (const frame of frames) {
      const mapId = pluginRadarMapId(frame.id);
      const sourceId = `${mapId}-source`;
      const layerId = `${mapId}-layer`;
      nextLayerIds.add(layerId);
      nextFrameVersions.set(layerId, frame.frame_time);
      const image = map.getSource(sourceId) as ImageSource | undefined;
      if (image && pluginRadarFrameVersions.get(layerId) !== frame.frame_time) {
        image.updateImage({ url: frame.url, coordinates: frame.coordinates });
      } else if (!image) {
        map.addSource(sourceId, {
          type: "image",
          url: frame.url,
          coordinates: frame.coordinates,
        });
        map.addLayer(
          {
            id: layerId,
            type: "raster",
            source: sourceId,
            layout: { visibility: pluginWeatherVisible ? "visible" : "none" },
            paint: {
              "raster-opacity": lowResource ? 0.42 : 0.58,
              "raster-fade-duration": prefersReducedMotion ? 0 : 300,
            },
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
    for (const layerId of pluginRadarLayerIds) {
      if (nextLayerIds.has(layerId)) continue;
      const sourceId = `${layerId.slice(0, -"-layer".length)}-source`;
      if (map.getLayer(layerId)) map.removeLayer(layerId);
      if (map.getSource(sourceId)) map.removeSource(sourceId);
    }
    pluginRadarLayerIds = nextLayerIds;
    pluginRadarFrameVersions = nextFrameVersions;
  }

  function updateAtlas(): void {
    if (!map || !mapReady) return;

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
    const routes = routeLineFeatures(flightRoute);
    const routeMarkers = routeMarkerFeatures(flightRoute);
    const weatherStations = weatherStationFeatures(weather);
    const weatherWinds = weatherWindFeatures(weather);
    const dispatchRouteData = atlasRouteGeoJson(route);
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
    map.setLayoutProperty(
      PLUGIN_WEATHER_GRID_LAYER_ID,
      "visibility",
      pluginWeatherVisibility,
    );
    map.setLayoutProperty(
      PLUGIN_WEATHER_ATMOSPHERE_LAYER_ID,
      "visibility",
      pluginWeatherVisible && enhancedWeather && !lowResource
        ? "visible"
        : "none",
    );
    const weatherVisibility = weatherVisible ? "visible" : "none";
    const gpuWeatherVisibility =
      weatherVisible && enhancedWeather && !lowResource ? "visible" : "none";
    map.setLayoutProperty(
      WEATHER_ATMOSPHERE_LAYER_ID,
      "visibility",
      gpuWeatherVisibility,
    );
    map.setLayoutProperty(
      WEATHER_EFFECT_LAYER_ID,
      "visibility",
      gpuWeatherVisibility,
    );
    map.setLayoutProperty(
      WEATHER_WIND_LAYER_ID,
      "visibility",
      gpuWeatherVisibility,
    );
    map.setLayoutProperty(
      WEATHER_WIND_TIP_LAYER_ID,
      "visibility",
      gpuWeatherVisibility,
    );
    map.setLayoutProperty(WEATHER_LAYER_ID, "visibility", weatherVisibility);
    map.setLayoutProperty(
      WEATHER_LABEL_LAYER_ID,
      "visibility",
      weatherVisibility,
    );
    const routeVisibility = routeVisible ? "visible" : "none";
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
      $activeTheme.colors.accent,
    ]);
    map.setPaintProperty(WEATHER_EFFECT_LAYER_ID, "circle-stroke-color", [
      "match",
      ["get", "effect"],
      "convective",
      $activeTheme.colors.danger,
      $activeTheme.colors.highlight,
    ]);
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
      weatherStations.features.some(
        ({ properties }) => properties.effect !== "none",
      ),
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
    enhancedWeather;
    regionsVisible;
    lowResource;
    selectedRegionId;
    prefersReducedMotion;
    selectedRoutePointId;
    selectedWeatherStationId;
    route;
    routeVisible;
    selectedAircraftId;
    selectedFboId;
    selectedRouteFeatureId;
    $activeTheme;
    updateAtlas();
  });

  $effect(() => {
    mapReady;
    focusRequest;
    route;
    applyFocusRequest();
  });

  onMount(() => {
    let cancelled = false;
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
        center: ATLAS_HOME_CENTER,
        zoom: 1.25,
        attributionControl: false,
      });
      map = atlasMap;

      atlasMap.addControl(
        new maplibregl.NavigationControl({ visualizePitch: true }),
        "top-right",
      );
      atlasMap.addControl(
        new maplibregl.AttributionControl({ compact: true }),
        "bottom-right",
      );

      atlasMap.on("load", () => {
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
        atlasMap.addLayer({
          id: PLUGIN_WEATHER_ATMOSPHERE_LAYER_ID,
          type: "circle",
          source: PLUGIN_WEATHER_GRID_SOURCE_ID,
          layout: {
            visibility:
              pluginWeatherVisible && enhancedWeather && !lowResource
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
          id: WEATHER_ATMOSPHERE_LAYER_ID,
          type: "heatmap",
          source: WEATHER_SOURCE_ID,
          filter: ["==", ["get", "has_metar"], true],
          layout: {
            visibility:
              weatherVisible && enhancedWeather && !lowResource
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
          id: WEATHER_EFFECT_LAYER_ID,
          type: "circle",
          source: WEATHER_SOURCE_ID,
          filter: ["!=", ["get", "effect"], "none"],
          layout: {
            visibility:
              weatherVisible && enhancedWeather && !lowResource
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
          id: WEATHER_WIND_LAYER_ID,
          type: "line",
          source: WEATHER_WIND_SOURCE_ID,
          filter: ["==", ["get", "feature_type"], "wind_path"],
          layout: {
            visibility:
              weatherVisible && enhancedWeather && !lowResource
                ? "visible"
                : "none",
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
              weatherVisible && enhancedWeather && !lowResource
                ? "visible"
                : "none",
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
      motionQuery.removeEventListener("change", updateMotionPreference);
      stopWeatherAnimation();
      clearHoveredRegion();
      map?.remove();
    };
  });
</script>

<div bind:this={mapContainer} class="map" aria-label="Atlas map"></div>

{#if weatherVisible && plottedWeatherStationCount > 0}
  <div class="weather-render-status" role="status">
    <span
      >{lowResource || !enhancedWeather
        ? "Weather fallback"
        : "GPU weather"}</span
    >
    <strong>
      {plottedWeatherStationCount} sourced
      {plottedWeatherStationCount === 1 ? "station" : "stations"}
    </strong>
    <small>
      {#if lowResource}
        Markers only · low-resource mode
      {:else if !enhancedWeather}
        Markers only · compatibility preference
      {:else if prefersReducedMotion}
        Static motion-safe atmosphere · {plottedWeatherWindCount} wind vectors
      {:else}
        {plottedWeatherWindCount} wind vectors · {activeWeatherEffectCount}
        condition cells
      {/if}
    </small>
    <em>METAR-local only · no interpolated weather</em>
  </div>
{/if}

<style>
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
  }
</style>
