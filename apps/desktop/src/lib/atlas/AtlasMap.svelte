<script lang="ts">
  import type { GeoJSONSource, Map } from "maplibre-gl";
  import "maplibre-gl/dist/maplibre-gl.css";
  import { onMount } from "svelte";
  import { activeTheme } from "$lib/theme/runtime";
  import type { AircraftSummary, FboSummary } from "./types";

  let {
    aircraft,
    fbos,
    fleetVisible,
    fboVisible,
    selectedAircraftId,
    selectedFboId,
    onselectaircraft,
    onselectfbo,
  }: {
    aircraft: AircraftSummary[];
    fbos: FboSummary[];
    fleetVisible: boolean;
    fboVisible: boolean;
    selectedAircraftId: string | null;
    selectedFboId: string | null;
    onselectaircraft: (aircraftId: string) => void;
    onselectfbo: (fboId: string) => void;
  } = $props();

  const FLEET_SOURCE_ID = "wyrmgrid-fleet";
  const FLEET_LAYER_ID = "wyrmgrid-fleet-aircraft";
  const FLEET_LABEL_LAYER_ID = "wyrmgrid-fleet-labels";
  const FBO_SOURCE_ID = "wyrmgrid-fbos";
  const FBO_LAYER_ID = "wyrmgrid-fbo-network";
  const FBO_LABEL_LAYER_ID = "wyrmgrid-fbo-labels";

  let mapContainer: HTMLDivElement;
  let map: Map | undefined;
  let mapReady = $state(false);
  let fittedAtlasSignature = "";

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
                  coordinates: [item.location.longitude, item.location.latitude] as [
                    number,
                    number,
                  ],
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
          ? [{
              type: "Feature" as const,
              geometry: {
                type: "Point" as const,
                coordinates: [
                  item.airport.location.longitude,
                  item.airport.location.latitude,
                ] as [number, number],
              },
              properties: { id: item.id, name: item.name, icao: item.airport.icao },
            }]
          : [],
      ),
    };
  }

  function updateAtlas(): void {
    if (!map || !mapReady) return;

    const fleet = fleetFeatures();
    const fboNetwork = fboFeatures();
    (map.getSource(FLEET_SOURCE_ID) as GeoJSONSource | undefined)?.setData(fleet);
    (map.getSource(FBO_SOURCE_ID) as GeoJSONSource | undefined)?.setData(fboNetwork);

    const visibility = fleetVisible ? "visible" : "none";
    map.setLayoutProperty(FLEET_LAYER_ID, "visibility", visibility);
    map.setLayoutProperty(FLEET_LABEL_LAYER_ID, "visibility", visibility);
    const fboVisibility = fboVisible ? "visible" : "none";
    map.setLayoutProperty(FBO_LAYER_ID, "visibility", fboVisibility);
    map.setLayoutProperty(FBO_LABEL_LAYER_ID, "visibility", fboVisibility);
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
    map.setPaintProperty(FLEET_LAYER_ID, "circle-stroke-color", $activeTheme.colors.map_halo);
    map.setPaintProperty(FBO_LAYER_ID, "circle-stroke-color", $activeTheme.colors.map_halo);
    map.setPaintProperty(FLEET_LABEL_LAYER_ID, "text-color", $activeTheme.colors.map_label);
    map.setPaintProperty(FBO_LABEL_LAYER_ID, "text-color", $activeTheme.colors.map_label);
    map.setPaintProperty(FLEET_LABEL_LAYER_ID, "text-halo-color", $activeTheme.colors.map_halo);
    map.setPaintProperty(FBO_LABEL_LAYER_ID, "text-halo-color", $activeTheme.colors.map_halo);

    const visibleFeatures = [
      ...(fleetVisible ? fleet.features : []),
      ...(fboVisible ? fboNetwork.features : []),
    ];
    const signature = visibleFeatures
      .map((feature) => `${feature.properties.id}:${feature.geometry.coordinates.join(",")}`)
      .sort()
      .join("|");
    if (!signature || signature === fittedAtlasSignature) return;

    fittedAtlasSignature = signature;
    const coordinates = visibleFeatures.map((feature) => feature.geometry.coordinates);
    const bounds = coordinates.reduce(
      (current, coordinate) => current.extend(coordinate),
      new maplibregl.LngLatBounds(coordinates[0], coordinates[0]),
    );
    map.fitBounds(bounds, { padding: 90, maxZoom: 6, duration: 700 });
  }

  let maplibregl: typeof import("maplibre-gl");

  $effect(() => {
    aircraft;
    fbos;
    fleetVisible;
    fboVisible;
    selectedAircraftId;
    selectedFboId;
    $activeTheme;
    updateAtlas();
  });

  onMount(() => {
    let cancelled = false;

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
        atlasMap.addSource(FLEET_SOURCE_ID, {
          type: "geojson",
          data: fleetFeatures(),
        });
        atlasMap.addSource(FBO_SOURCE_ID, {
          type: "geojson",
          data: fboFeatures(),
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
          minzoom: 3,
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
          minzoom: 3,
          layout: {
            "text-field": ["coalesce", ["get", "registration"], ["get", "model"], "Aircraft"],
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
        atlasMap.on("click", FLEET_LAYER_ID, (event) => {
          const aircraftId = event.features?.[0]?.properties?.id;
          if (typeof aircraftId === "string") onselectaircraft(aircraftId);
        });
        atlasMap.on("click", FBO_LAYER_ID, (event) => {
          const fboId = event.features?.[0]?.properties?.id;
          if (typeof fboId === "string") onselectfbo(fboId);
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
        mapReady = true;
        updateAtlas();
      });
    });

    return () => {
      cancelled = true;
      map?.remove();
    };
  });
</script>

<div bind:this={mapContainer} class="map" aria-label="Atlas map"></div>
