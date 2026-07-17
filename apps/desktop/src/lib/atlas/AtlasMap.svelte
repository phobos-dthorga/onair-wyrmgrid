<script lang="ts">
  import type {
    ExpressionSpecification,
    GeoJSONSource,
    Map,
  } from "maplibre-gl";
  import "maplibre-gl/dist/maplibre-gl.css";
  import { onMount } from "svelte";
  import type { AtlasRouteView } from "$lib/dispatch/types";
  import type { PublishedPluginLayer } from "$lib/forge/types";
  import { activeTheme } from "$lib/theme/runtime";
  import {
    weatherFitCoordinates,
    weatherMapSignature,
    weatherPointCoordinates,
    weatherStationFeatures,
  } from "$lib/weather/atlasWeather";
  import type { FlightWeatherMapView } from "$lib/weather/types";
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
    flightRoute,
    weather,
    weatherVisible,
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
    flightRoute?: AtlasFlightRoute;
    weather?: FlightWeatherMapView;
    weatherVisible: boolean;
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
  const ROUTE_SOURCE_ID = "wyrmgrid-flight-routes";
  const ROUTE_MARKER_SOURCE_ID = "wyrmgrid-flight-route-markers";
  const PLANNED_ROUTE_LAYER_ID = "wyrmgrid-planned-flight-route";
  const RECORDED_ROUTE_LAYER_ID = "wyrmgrid-recorded-flight-route";
  const ROUTE_MARKER_LAYER_ID = "wyrmgrid-flight-route-markers";
  const ROUTE_LABEL_LAYER_ID = "wyrmgrid-flight-route-labels";
  const WEATHER_SOURCE_ID = "wyrmgrid-flight-weather";
  const WEATHER_LAYER_ID = "wyrmgrid-flight-weather-stations";
  const WEATHER_LABEL_LAYER_ID = "wyrmgrid-flight-weather-labels";
  const DISPATCH_ROUTE_SOURCE_ID = "wyrmgrid-dispatch-route";
  const DISPATCH_ROUTE_LINE_LAYER_ID = "wyrmgrid-dispatch-route-line";
  const DISPATCH_ROUTE_POINT_LAYER_ID = "wyrmgrid-dispatch-route-points";
  const DISPATCH_ROUTE_LABEL_LAYER_ID = "wyrmgrid-dispatch-route-labels";

  let mapContainer: HTMLDivElement;
  let map: Map | undefined;
  let mapReady = $state(false);
  let fittedAtlasSignature = "";
  let hoveredRegionFeatureId: string | number | undefined;
  let selectedRegionFeatureId: string | number | undefined;
  let prefersReducedMotion = $state(false);

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
    const routes = routeLineFeatures(flightRoute);
    const routeMarkers = routeMarkerFeatures(flightRoute);
    const weatherStations = weatherStationFeatures(weather);
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
    const weatherVisibility = weatherVisible ? "visible" : "none";
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
    const bounds = visibleCoordinates.reduce(
      (current, coordinate) => current.extend(coordinate),
      new maplibregl.LngLatBounds(
        visibleCoordinates[0],
        visibleCoordinates[0],
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
    flightRoute;
    weather;
    weatherVisible;
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
        center: [18, 22],
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
              "symbol-sort-key": [
                "coalesce",
                ["get", "label_rank"],
                99,
              ],
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
      clearHoveredRegion();
      map?.remove();
    };
  });
</script>

<div bind:this={mapContainer} class="map" aria-label="Atlas map"></div>
