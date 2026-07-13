<script lang="ts">
  import type { GeoJSONSource, Map } from "maplibre-gl";
  import "maplibre-gl/dist/maplibre-gl.css";
  import { onMount } from "svelte";
  import type { AircraftSummary } from "./types";

  let {
    aircraft,
    fleetVisible,
    selectedAircraftId,
    onselect,
  }: {
    aircraft: AircraftSummary[];
    fleetVisible: boolean;
    selectedAircraftId: string | null;
    onselect: (aircraftId: string) => void;
  } = $props();

  const FLEET_SOURCE_ID = "wyrmgrid-fleet";
  const FLEET_LAYER_ID = "wyrmgrid-fleet-aircraft";
  const FLEET_LABEL_LAYER_ID = "wyrmgrid-fleet-labels";

  let mapContainer: HTMLDivElement;
  let map: Map | undefined;
  let mapReady = $state(false);
  let fittedFleetSignature = "";

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

  function updateFleet(): void {
    if (!map || !mapReady) return;

    const features = fleetFeatures();
    const source = map.getSource(FLEET_SOURCE_ID) as GeoJSONSource | undefined;
    source?.setData(features);

    const visibility = fleetVisible ? "visible" : "none";
    map.setLayoutProperty(FLEET_LAYER_ID, "visibility", visibility);
    map.setLayoutProperty(FLEET_LABEL_LAYER_ID, "visibility", visibility);
    map.setPaintProperty(FLEET_LAYER_ID, "circle-color", [
      "case",
      ["==", ["get", "id"], selectedAircraftId ?? ""],
      "#d5ae5f",
      "#73d6ad",
    ]);

    const signature = features.features
      .map((feature) => `${feature.properties.id}:${feature.geometry.coordinates.join(",")}`)
      .sort()
      .join("|");
    if (!signature || signature === fittedFleetSignature) return;

    fittedFleetSignature = signature;
    const coordinates = features.features.map((feature) => feature.geometry.coordinates);
    const bounds = coordinates.reduce(
      (current, coordinate) => current.extend(coordinate),
      new maplibregl.LngLatBounds(coordinates[0], coordinates[0]),
    );
    map.fitBounds(bounds, { padding: 90, maxZoom: 6, duration: 700 });
  }

  let maplibregl: typeof import("maplibre-gl");

  $effect(() => {
    aircraft;
    fleetVisible;
    selectedAircraftId;
    updateFleet();
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
        atlasMap.addLayer({
          id: FLEET_LAYER_ID,
          type: "circle",
          source: FLEET_SOURCE_ID,
          paint: {
            "circle-radius": ["interpolate", ["linear"], ["zoom"], 1, 5, 7, 9],
            "circle-color": "#73d6ad",
            "circle-stroke-color": "#07110f",
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
            "text-color": "#e9f1ef",
            "text-halo-color": "#07110f",
            "text-halo-width": 1.5,
          },
        });
        atlasMap.on("click", FLEET_LAYER_ID, (event) => {
          const aircraftId = event.features?.[0]?.properties?.id;
          if (typeof aircraftId === "string") onselect(aircraftId);
        });
        atlasMap.on("mouseenter", FLEET_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "pointer";
        });
        atlasMap.on("mouseleave", FLEET_LAYER_ID, () => {
          atlasMap.getCanvas().style.cursor = "";
        });
        mapReady = true;
        updateFleet();
      });
    });

    return () => {
      cancelled = true;
      map?.remove();
    };
  });
</script>

<div bind:this={mapContainer} class="map" aria-label="Atlas map"></div>
