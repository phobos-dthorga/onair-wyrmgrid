import * as THREE from "three/webgpu";
import {
  Break,
  Fn,
  If,
  cameraPosition,
  float,
  modelWorldMatrixInverse,
  positionGeometry,
  screenSize,
  screenUV,
  smoothstep,
  texture3D,
  uniform,
  varying,
  vec2,
  vec3,
  vec4,
} from "three/tsl";
import { RaymarchingBox } from "three/addons/tsl/utils/Raymarching.js";
import { AdaptiveWeatherQualityController } from "./adaptiveQuality";
import { deterministicWeatherUnit, hashWeatherText } from "./deterministic";
import {
  adaptWeatherRenderBudget,
  resolveWeatherRenderBudget,
  type AdaptiveWeatherQuality,
  type WeatherRenderBudget,
} from "./quality";
import type {
  WeatherRenderer,
  WeatherRendererBackend,
  WeatherRendererFrame,
  WeatherRendererUpdate,
} from "./types";
import { weatherVisualSurfaceVisibility } from "./surfaceClipping";
import { weatherVolumeVariation } from "./volumeVariation";
import type { WeatherRenderCell } from "./weatherRenderScene";
import {
  PRECIPITATION_FIELD_HEIGHT,
  PRECIPITATION_FIELD_WIDTH,
  precipitationParticleTaper,
  precipitationVerticalPosition,
} from "./precipitationLayout";
import {
  generateWeatherVolumeDensityAsync,
  WEATHER_VOLUME_TEXTURE_SIZE,
} from "./volumeDensity";
import {
  resolveWeatherVolumeAppearance,
  weatherCloudColor,
  type WeatherVolumeAppearanceOverrides,
} from "./volumeAppearance";

type ParticleSeed = {
  x: number;
  y: number;
  z: number;
  phase: number;
  speed: number;
};

type CloudPuff = {
  mesh: THREE.Mesh;
  baseX: number;
  baseY: number;
  phase: number;
};

type WeatherVolume = {
  mesh: THREE.Mesh;
  baseRotation: number;
  phase: number;
};

type CellVisual = {
  cell: WeatherRenderCell;
  group: THREE.Group;
  clouds: CloudPuff[];
  volume?: WeatherVolume;
  precipitation?: THREE.InstancedMesh;
  precipitationSeeds: ParticleSeed[];
  dust?: THREE.Points;
  lightning?: THREE.Line;
  lightningMaterial?: THREE.LineBasicMaterial;
  surfaceSample?: {
    projectionKey: string;
    x: number;
    y: number;
    radius: number;
    visibility: number;
  };
};

const SCREEN_MARGIN = 210;
const MINIMUM_SURFACE_VISIBILITY = 0.01;
const DEGREES_TO_RADIANS = Math.PI / 180;
const WEATHER_VISUAL_RADIUS = 126;
const WEATHER_CAMERA_VERTICAL_FOV_DEGREES = 32;

function disposeObject(root: THREE.Object3D): void {
  const geometries = new Set<THREE.BufferGeometry>();
  const materials = new Set<THREE.Material>();
  root.traverse((object) => {
    const rendered = object as THREE.Mesh | THREE.Points | THREE.Line;
    if (rendered.geometry instanceof THREE.BufferGeometry) {
      geometries.add(rendered.geometry);
    }
    const material = rendered.material;
    if (Array.isArray(material)) {
      material.forEach((entry) => materials.add(entry));
    } else if (material instanceof THREE.Material) {
      materials.add(material);
    }
  });
  geometries.forEach((geometry) => geometry.dispose());
  materials.forEach((material) => material.dispose());
}

function registerWeatherMaterial(material: THREE.Material): void {
  material.userData.weatherBaseOpacity = material.opacity;
}

function applySurfaceVisibility(
  root: THREE.Object3D,
  visibility: number,
): void {
  root.traverse((object) => {
    const rendered = object as THREE.Mesh | THREE.Points | THREE.Line;
    const entries = Array.isArray(rendered.material)
      ? rendered.material
      : rendered.material instanceof THREE.Material
        ? [rendered.material]
        : [];
    for (const material of entries) {
      const baseOpacity = material.userData.weatherBaseOpacity;
      if (typeof baseOpacity === "number") {
        material.opacity = baseOpacity * visibility;
      }
    }
  });
}

async function createVolumeDensityTexture(): Promise<THREE.Data3DTexture> {
  const density = await generateWeatherVolumeDensityAsync();
  const texture = new THREE.Data3DTexture(
    density,
    WEATHER_VOLUME_TEXTURE_SIZE,
    WEATHER_VOLUME_TEXTURE_SIZE,
    WEATHER_VOLUME_TEXTURE_SIZE,
  );
  texture.format = THREE.RedFormat;
  texture.minFilter = THREE.LinearFilter;
  texture.magFilter = THREE.LinearFilter;
  texture.unpackAlignment = 1;
  texture.needsUpdate = true;
  return texture;
}

function createVolumeMaterial(
  texture: THREE.Data3DTexture,
  cell: WeatherRenderCell,
  steps: number,
  variation: ReturnType<typeof weatherVolumeVariation>,
  appearanceOverrides: WeatherVolumeAppearanceOverrides,
): THREE.NodeMaterial {
  const densityTexture = texture3D(texture, null, 0);
  const dust = cell.effect === "dust";
  const appearance = resolveWeatherVolumeAppearance(
    cell.effect,
    cell.intensity,
    steps,
    {
      ...appearanceOverrides,
      thresholdOffset:
        (appearanceOverrides.thresholdOffset ?? 0) +
        variation.densityThresholdOffset,
    },
  );
  const threshold = float(appearance.threshold);
  const range = float(appearance.transitionRange);
  const opacity = float(appearance.sampleOpacity);
  const baseColor = uniform(new THREE.Color(appearance.color));
  const localRayOrigin = varying(
    modelWorldMatrixInverse.mul(vec4(cameraPosition, 1)).xyz,
  );
  const localRayDirection = varying(
    positionGeometry.sub(localRayOrigin),
  ).normalize();
  const screenPixel = screenUV.mul(screenSize).floor();
  const raySampleOffset = screenPixel
    .dot(vec2(0.06711056, 0.00583715))
    .fract()
    .mul(52.9829189)
    .fract()
    .sub(0.5)
    .mul(0.9 / steps);
  const volumeNode = Fn(() => {
    const finalColor = vec4(0).toVar();
    RaymarchingBox(steps, ({ positionRay }) => {
      const jittered = positionRay.add(localRayDirection.mul(raySampleOffset));
      const cosX = Math.cos(variation.sampleRotation.x);
      const sinX = Math.sin(variation.sampleRotation.x);
      const cosY = Math.cos(variation.sampleRotation.y);
      const sinY = Math.sin(variation.sampleRotation.y);
      const cosZ = Math.cos(variation.sampleRotation.z);
      const sinZ = Math.sin(variation.sampleRotation.z);
      const aroundX = vec3(
        jittered.x,
        jittered.y.mul(cosX).sub(jittered.z.mul(sinX)),
        jittered.y.mul(sinX).add(jittered.z.mul(cosX)),
      );
      const aroundY = vec3(
        aroundX.x.mul(cosY).add(aroundX.z.mul(sinY)),
        aroundX.y,
        aroundX.z.mul(cosY).sub(aroundX.x.mul(sinY)),
      );
      const rotated = vec3(
        aroundY.x.mul(cosZ).sub(aroundY.y.mul(sinZ)),
        aroundY.x.mul(sinZ).add(aroundY.y.mul(cosZ)),
        aroundY.z,
      );
      const coordinates = rotated
        .add(
          vec3(
            variation.sampleOffset.x,
            variation.sampleOffset.y,
            variation.sampleOffset.z,
          ),
        )
        .add(0.5);
      const mapValue = float(densityTexture.sample(coordinates).r).toVar();
      mapValue.assign(
        smoothstep(threshold.sub(range), threshold.add(range), mapValue).mul(
          opacity,
        ),
      );
      const shading = densityTexture
        .sample(coordinates.sub(vec3(-0.018, 0.024, 0.012)))
        .r.sub(densityTexture.sample(coordinates.add(vec3(0.015))).r);
      const light = shading
        .mul(dust ? 2.2 : 3.8)
        .add(positionRay.y.mul(dust ? -0.08 : -0.28))
        .add(dust ? 0.76 : 0.9)
        .clamp(dust ? 0.38 : 0.46, dust ? 1.18 : 1.38);
      const contribution = finalColor.a
        .oneMinus()
        .mul(mapValue)
        .mul(baseColor.mul(light));
      finalColor.rgb.addAssign(contribution);
      finalColor.a.addAssign(finalColor.a.oneMinus().mul(mapValue));
      If(finalColor.a.greaterThanEqual(0.96), () => {
        Break();
      });
    });
    return finalColor;
  })();
  const material = new THREE.NodeMaterial();
  material.colorNode = volumeNode;
  material.side = THREE.BackSide;
  material.transparent = true;
  material.depthTest = false;
  material.depthWrite = false;
  material.premultipliedAlpha = true;
  material.toneMapped = false;
  registerWeatherMaterial(material);
  return material;
}

function addVolume(
  visual: CellVisual,
  texture: THREE.Data3DTexture,
  steps: number,
  appearanceOverrides: WeatherVolumeAppearanceOverrides,
): void {
  const variation = weatherVolumeVariation(visual.cell.id);
  const appearance = resolveWeatherVolumeAppearance(
    visual.cell.effect,
    visual.cell.intensity,
    steps,
    appearanceOverrides,
  );
  const mesh = new THREE.Mesh(
    new THREE.BoxGeometry(1, 1, 1),
    createVolumeMaterial(
      texture,
      visual.cell,
      steps,
      variation,
      appearanceOverrides,
    ),
  );
  mesh.position.y = appearance.verticalOffset;
  mesh.scale.set(
    appearance.scale.x * variation.scale.x,
    appearance.scale.y * variation.scale.y,
    appearance.scale.z * variation.scale.z,
  );
  const phase =
    deterministicWeatherUnit(
      hashWeatherText(`${visual.cell.id}:volume-animation`),
      0,
    ) *
    Math.PI *
    2;
  const baseRotation = variation.meshRotationRadians;
  mesh.rotation.z = baseRotation;
  visual.volume = { mesh, baseRotation, phase };
  visual.group.add(mesh);
}

function addClouds(
  visual: CellVisual,
  count: number,
  cinematic: boolean,
): void {
  const seed = hashWeatherText(`${visual.cell.id}:cloud`);
  for (let index = 0; index < count; index += 1) {
    const size = 13 + deterministicWeatherUnit(seed, index * 5) * 11;
    const geometry = new THREE.SphereGeometry(size, cinematic ? 14 : 10, 8);
    const material = new THREE.MeshPhongMaterial({
      color: weatherCloudColor(visual.cell.effect),
      depthTest: false,
      depthWrite: false,
      opacity: (cinematic ? 0.2 : 0.16) * (0.72 + visual.cell.intensity * 0.28),
      shininess: 9,
      transparent: true,
    });
    registerWeatherMaterial(material);
    const mesh = new THREE.Mesh(geometry, material);
    const baseX = (deterministicWeatherUnit(seed, index * 5 + 1) - 0.5) * 70;
    const baseY =
      (deterministicWeatherUnit(seed, index * 5 + 2) - 0.5) * 35 - 15;
    const depth = (deterministicWeatherUnit(seed, index * 5 + 3) - 0.5) * 30;
    const stretch = 0.8 + deterministicWeatherUnit(seed, index * 5 + 4) * 0.8;
    mesh.position.set(baseX, baseY, depth);
    mesh.scale.set(stretch, 0.58 + visual.cell.intensity * 0.18, 0.72);
    visual.group.add(mesh);
    visual.clouds.push({
      mesh,
      baseX,
      baseY,
      phase: deterministicWeatherUnit(seed, index * 7 + 6) * Math.PI * 2,
    });
  }
}

function addPrecipitation(
  visual: CellVisual,
  count: number,
  snow: boolean,
): void {
  const geometry = snow
    ? new THREE.CircleGeometry(1.35, 7)
    : new THREE.PlaneGeometry(1.15, 15);
  const material = new THREE.MeshBasicMaterial({
    color: snow ? 0xe8f7ff : 0x8ed9ff,
    depthTest: false,
    depthWrite: false,
    opacity: snow ? 0.78 : 0.66,
    side: THREE.DoubleSide,
    transparent: true,
  });
  registerWeatherMaterial(material);
  const precipitation = new THREE.InstancedMesh(geometry, material, count);
  precipitation.frustumCulled = false;
  const seed = hashWeatherText(`${visual.cell.id}:precipitation`);
  for (let index = 0; index < count; index += 1) {
    visual.precipitationSeeds.push({
      x:
        (deterministicWeatherUnit(seed, index * 5) - 0.5) *
        PRECIPITATION_FIELD_WIDTH,
      y:
        deterministicWeatherUnit(seed, index * 5 + 1) *
        PRECIPITATION_FIELD_HEIGHT,
      z: (deterministicWeatherUnit(seed, index * 5 + 2) - 0.5) * 24,
      phase: deterministicWeatherUnit(seed, index * 5 + 3) * Math.PI * 2,
      speed:
        (snow ? 9 : 42) +
        deterministicWeatherUnit(seed, index * 5 + 4) * (snow ? 8 : 25),
    });
  }
  visual.precipitation = precipitation;
  visual.group.add(precipitation);
}

function addDust(visual: CellVisual, count: number): void {
  const seed = hashWeatherText(`${visual.cell.id}:dust`);
  const positions = new Float32Array(count * 3);
  for (let index = 0; index < count; index += 1) {
    positions[index * 3] =
      (deterministicWeatherUnit(seed, index * 3) - 0.5) * 105;
    positions[index * 3 + 1] =
      (deterministicWeatherUnit(seed, index * 3 + 1) - 0.5) * 68;
    positions[index * 3 + 2] =
      (deterministicWeatherUnit(seed, index * 3 + 2) - 0.5) * 25;
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  const material = new THREE.PointsMaterial({
    color: 0xc79b62,
    depthTest: false,
    depthWrite: false,
    opacity: 0.48 + visual.cell.intensity * 0.18,
    size: 2.6,
    sizeAttenuation: false,
    transparent: true,
  });
  registerWeatherMaterial(material);
  visual.dust = new THREE.Points(geometry, material);
  visual.dust.frustumCulled = false;
  visual.group.add(visual.dust);
}

function addLightning(visual: CellVisual): void {
  const seed = hashWeatherText(`${visual.cell.id}:lightning`);
  const points: THREE.Vector3[] = [];
  let x = 8;
  for (let index = 0; index < 8; index += 1) {
    const y = -42 + index * 12;
    x += (deterministicWeatherUnit(seed, index) - 0.5) * 14;
    points.push(new THREE.Vector3(x, y, 40));
  }
  const geometry = new THREE.BufferGeometry().setFromPoints(points);
  const material = new THREE.LineBasicMaterial({
    color: 0xffe79a,
    depthTest: false,
    depthWrite: false,
    opacity: 0.45,
    transparent: true,
  });
  registerWeatherMaterial(material);
  visual.lightning = new THREE.Line(geometry, material);
  visual.lightningMaterial = material;
  visual.group.add(visual.lightning);
}

function createCellVisual(
  cell: WeatherRenderCell,
  update: WeatherRendererUpdate,
  budget: WeatherRenderBudget,
  volumeTexture: THREE.Data3DTexture | undefined,
  useVolume: boolean,
  appearanceOverrides: WeatherVolumeAppearanceOverrides,
): CellVisual | undefined {
  const visual: CellVisual = {
    cell,
    group: new THREE.Group(),
    clouds: [],
    precipitationSeeds: [],
  };
  const cloudBearingEffect = ["cloud", "rain", "snow", "convective"].includes(
    cell.effect,
  );
  if (
    (update.policy.clouds && cloudBearingEffect) ||
    cell.effect === "obscuration"
  ) {
    if (useVolume && volumeTexture) {
      addVolume(
        visual,
        volumeTexture,
        budget.volumeRaymarchSteps,
        appearanceOverrides,
      );
    } else {
      addClouds(
        visual,
        budget.cloudPuffsPerCell,
        update.policy.profile === "cinematic",
      );
    }
  }
  if (
    update.policy.precipitation &&
    ["rain", "snow", "convective"].includes(cell.effect)
  ) {
    addPrecipitation(
      visual,
      budget.precipitationParticlesPerCell,
      cell.effect === "snow",
    );
  }
  if (update.policy.dust && cell.effect === "dust") {
    if (useVolume && volumeTexture) {
      addVolume(
        visual,
        volumeTexture,
        budget.volumeRaymarchSteps,
        appearanceOverrides,
      );
    }
    addDust(visual, budget.dustParticlesPerCell);
  }
  if (update.policy.lightning && cell.effect === "convective") {
    addLightning(visual);
  }
  return visual.group.children.length > 0 ? visual : undefined;
}

class ThreeWeatherRenderer implements WeatherRenderer {
  readonly backend: WeatherRendererBackend;

  private readonly scene = new THREE.Scene();
  private readonly camera = new THREE.PerspectiveCamera(
    WEATHER_CAMERA_VERTICAL_FOV_DEGREES,
    1,
    0.1,
    4_000,
  );
  private readonly root = new THREE.Group();
  private readonly instancePosition = new THREE.Vector3();
  private readonly instanceRotation = new THREE.Quaternion();
  private readonly instanceScale = new THREE.Vector3(1, 1, 1);
  private readonly instanceMatrix = new THREE.Matrix4();
  private readonly screenAxis = new THREE.Vector3(0, 0, 1);
  private readonly adaptiveQuality = new AdaptiveWeatherQualityController();
  private readonly volumeTexture: THREE.Data3DTexture | undefined;
  private activeCells: CellVisual[] = [];
  private updateValue: WeatherRendererUpdate;
  private visibleSignature = "";
  private width = 0;
  private height = 0;
  private pixelRatio = 0;

  constructor(
    private readonly renderer: THREE.WebGPURenderer,
    initialUpdate: WeatherRendererUpdate,
    backend: WeatherRendererBackend,
    volumeTexture: THREE.Data3DTexture | undefined,
    private readonly onQualityChanged: (
      quality: AdaptiveWeatherQuality,
    ) => void,
    private readonly appearanceOverrides: WeatherVolumeAppearanceOverrides,
  ) {
    this.backend = backend;
    this.volumeTexture = volumeTexture;
    this.updateValue = initialUpdate;
    this.root.scale.y = -1;
    this.scene.add(this.root);
    this.scene.add(new THREE.AmbientLight(0xd9e8ef, 1.25));
    const sunlight = new THREE.DirectionalLight(0xffffff, 2.2);
    sunlight.position.set(-80, -120, 160);
    this.scene.add(sunlight);
  }

  get quality(): AdaptiveWeatherQuality {
    return this.adaptiveQuality.quality;
  }

  update(update: WeatherRendererUpdate): void {
    const profileChanged =
      this.updateValue.policy.profile !== update.policy.profile;
    if (
      this.updateValue.scene.signature !== update.scene.signature ||
      this.updateValue.policy.profile !== update.policy.profile ||
      this.updateValue.policy.clouds !== update.policy.clouds ||
      this.updateValue.policy.precipitation !== update.policy.precipitation ||
      this.updateValue.policy.lightning !== update.policy.lightning ||
      this.updateValue.policy.dust !== update.policy.dust
    ) {
      this.visibleSignature = "";
    }
    if (profileChanged) {
      const previousQuality = this.adaptiveQuality.quality;
      this.adaptiveQuality.reset();
      if (previousQuality !== this.adaptiveQuality.quality) {
        this.onQualityChanged(this.adaptiveQuality.quality);
      }
    }
    this.updateValue = update;
  }

  render(frame: WeatherRendererFrame): void {
    const budget = adaptWeatherRenderBudget(
      resolveWeatherRenderBudget(this.updateValue.policy),
      this.adaptiveQuality.quality,
    );
    const nextPixelRatio = Math.min(
      Math.max(1, frame.pixelRatio),
      budget.maximumPixelRatio,
    );
    if (
      this.width !== frame.width ||
      this.height !== frame.height ||
      this.pixelRatio !== nextPixelRatio
    ) {
      this.width = frame.width;
      this.height = frame.height;
      this.pixelRatio = nextPixelRatio;
      this.renderer.setPixelRatio(nextPixelRatio);
      this.renderer.setSize(frame.width, frame.height, false);
      const cameraDistance =
        frame.height /
        (2 * Math.tan((WEATHER_CAMERA_VERTICAL_FOV_DEGREES * Math.PI) / 360));
      this.camera.aspect = frame.width / frame.height;
      this.camera.far = cameraDistance + 1_000;
      this.camera.position.set(
        frame.width / 2,
        frame.height / 2,
        cameraDistance,
      );
      this.camera.lookAt(frame.width / 2, frame.height / 2, 0);
      this.camera.updateProjectionMatrix();
      this.root.position.y = frame.height;
    }

    const projected = this.updateValue.scene.cells
      .map((cell) => ({
        cell,
        point: frame.project(cell.longitude, cell.latitude),
      }))
      .filter(
        ({ point }) =>
          point.surfaceVisibility > MINIMUM_SURFACE_VISIBILITY &&
          point.x >= -SCREEN_MARGIN &&
          point.x <= frame.width + SCREEN_MARGIN &&
          point.y >= -SCREEN_MARGIN &&
          point.y <= frame.height + SCREEN_MARGIN,
      )
      .sort(
        (left, right) =>
          Number(right.cell.source === "airport") -
            Number(left.cell.source === "airport") ||
          right.cell.intensity - left.cell.intensity,
      )
      .slice(0, budget.maximumCells);
    const signature = `${this.updateValue.scene.signature}:${this.updateValue.policy.profile}:${this.updateValue.policy.clouds}:${this.updateValue.policy.precipitation}:${this.updateValue.policy.lightning}:${this.updateValue.policy.dust}:${this.adaptiveQuality.quality}:${projected.map(({ cell }) => cell.id).join(",")}`;
    if (signature !== this.visibleSignature) {
      this.rebuild(
        projected.map(({ cell }) => cell),
        budget,
      );
      this.visibleSignature = signature;
    }

    const pointById = new Map(
      projected.map(({ cell, point }) => [cell.id, point]),
    );
    const scaleFromZoom = Math.min(
      1.45,
      Math.max(0.76, 0.72 + frame.zoom * 0.07),
    );
    for (const visual of this.activeCells) {
      const point = pointById.get(visual.cell.id);
      if (!point) {
        visual.group.visible = false;
        continue;
      }
      visual.group.visible = true;
      visual.group.position.set(point.x, point.y, 0);
      const intensityScale = 0.78 + visual.cell.intensity * 0.32;
      const visualScale = scaleFromZoom * intensityScale;
      visual.group.scale.setScalar(visualScale);
      const sampleRadius = WEATHER_VISUAL_RADIUS * visualScale;
      const cachedSample = visual.surfaceSample;
      const surfaceVisibility =
        cachedSample &&
        cachedSample.projectionKey === frame.projectionKey &&
        Math.abs(cachedSample.x - point.x) < 0.5 &&
        Math.abs(cachedSample.y - point.y) < 0.5 &&
        Math.abs(cachedSample.radius - sampleRadius) < 0.25
          ? cachedSample.visibility
          : weatherVisualSurfaceVisibility(
              point.surfaceVisibility,
              point.x,
              point.y,
              sampleRadius,
              frame.surfaceVisibilityAt,
            );
      visual.surfaceSample = {
        projectionKey: frame.projectionKey,
        x: point.x,
        y: point.y,
        radius: sampleRadius,
        visibility: surfaceVisibility,
      };
      if (surfaceVisibility <= MINIMUM_SURFACE_VISIBILITY) {
        visual.group.visible = false;
        continue;
      }
      this.animateCell(visual, frame);
      applySurfaceVisibility(visual.group, surfaceVisibility);
    }
    const submissionStart = performance.now();
    this.renderer.render(this.scene, this.camera);
    if (
      this.adaptiveQuality.recordSubmission(performance.now() - submissionStart)
    ) {
      this.visibleSignature = "";
      this.onQualityChanged(this.adaptiveQuality.quality);
    }
  }

  dispose(): void {
    this.clearCells();
    this.volumeTexture?.dispose();
    this.renderer.dispose();
  }

  private rebuild(
    cells: WeatherRenderCell[],
    budget: WeatherRenderBudget,
  ): void {
    this.clearCells();
    let volumeCellCount = 0;
    for (const cell of cells) {
      const supportsVolume =
        cell.effect === "dust" ||
        cell.effect === "obscuration" ||
        ["cloud", "rain", "snow", "convective"].includes(cell.effect);
      const useVolume =
        supportsVolume &&
        this.volumeTexture !== undefined &&
        volumeCellCount < budget.maximumVolumeCells;
      const visual = createCellVisual(
        cell,
        this.updateValue,
        budget,
        this.volumeTexture,
        useVolume,
        this.appearanceOverrides,
      );
      if (!visual) continue;
      if (visual.volume) volumeCellCount += 1;
      this.activeCells.push(visual);
      this.root.add(visual.group);
    }
  }

  private clearCells(): void {
    for (const visual of this.activeCells) {
      this.root.remove(visual.group);
      disposeObject(visual.group);
    }
    this.activeCells = [];
  }

  private animateCell(visual: CellVisual, frame: WeatherRendererFrame): void {
    const animationTime = this.updateValue.policy.animation ? frame.timeMs : 0;
    for (const cloud of visual.clouds) {
      cloud.mesh.position.x =
        cloud.baseX + Math.sin(animationTime / 3_400 + cloud.phase) * 3.5;
      cloud.mesh.position.y =
        cloud.baseY + Math.cos(animationTime / 4_200 + cloud.phase) * 1.8;
    }

    if (visual.volume) {
      const windRotation =
        visual.cell.windSpeedKt >= 2
          ? (visual.cell.windBearing - frame.bearing) * DEGREES_TO_RADIANS
          : 0;
      visual.volume.mesh.rotation.z =
        windRotation +
        visual.volume.baseRotation +
        Math.sin(animationTime / 8_200 + visual.volume.phase) * 0.025;
      visual.volume.mesh.position.x =
        Math.sin(animationTime / 5_900 + visual.volume.phase) * 2.6;
    }

    if (visual.precipitation) {
      const snow = visual.cell.effect === "snow";
      const windRotation =
        (visual.cell.windBearing - frame.bearing) * DEGREES_TO_RADIANS;
      this.instanceRotation.setFromAxisAngle(this.screenAxis, windRotation);
      const seconds = animationTime / 1_000;
      visual.precipitationSeeds.forEach((seed, index) => {
        const falling = precipitationVerticalPosition(
          seed.y,
          seconds,
          seed.speed,
        );
        const windDrift =
          Math.sin(seconds * 0.9 + seed.phase) *
          Math.min(12, visual.cell.windSpeedKt * 0.22);
        this.instancePosition.set(seed.x + windDrift, falling, seed.z);
        const taper = precipitationParticleTaper(seed.x + windDrift, falling);
        if (snow) {
          const pulse = 0.72 + Math.sin(seconds + seed.phase) * 0.22;
          this.instanceScale.setScalar(pulse * taper);
        } else {
          this.instanceScale.set(
            taper,
            (0.86 + visual.cell.intensity * 0.32) * taper,
            1,
          );
        }
        this.instanceMatrix.compose(
          this.instancePosition,
          this.instanceRotation,
          this.instanceScale,
        );
        visual.precipitation?.setMatrixAt(index, this.instanceMatrix);
      });
      visual.precipitation.instanceMatrix.needsUpdate = true;
    }

    if (visual.dust) {
      visual.dust.rotation.z = animationTime / 12_000;
      visual.dust.position.x = Math.sin(animationTime / 1_900) * 5;
      visual.dust.position.y = Math.cos(animationTime / 2_600) * 3;
    }

    if (visual.lightningMaterial) {
      let opacity: number;
      if (!this.updateValue.policy.lightningFlashes) {
        opacity = 0.45;
      } else {
        const phase =
          (animationTime + (hashWeatherText(visual.cell.id) % 4_200)) % 6_400;
        opacity = phase < 80 ? 1 : phase >= 150 && phase < 215 ? 0.72 : 0.08;
      }
      visual.lightningMaterial.userData.weatherBaseOpacity = opacity;
    }
  }
}

export async function createThreeWeatherRenderer(
  canvas: HTMLCanvasElement,
  initialUpdate: WeatherRendererUpdate,
  onDeviceLost: (backend: WeatherRendererBackend, reason: string) => void,
  onQualityChanged: (quality: AdaptiveWeatherQuality) => void,
  appearanceOverrides: WeatherVolumeAppearanceOverrides = {},
): Promise<WeatherRenderer> {
  const renderer = new THREE.WebGPURenderer({
    alpha: true,
    antialias: true,
    canvas,
    depth: true,
  });
  renderer.setClearColor(0x000000, 0);
  await renderer.init();
  const backend: WeatherRendererBackend =
    "isWebGPUBackend" in renderer.backend &&
    renderer.backend.isWebGPUBackend === true
      ? "webgpu"
      : "webgl2";
  const defaultDeviceLost = renderer.onDeviceLost.bind(renderer);
  renderer.onDeviceLost = (information) => {
    defaultDeviceLost(information);
    onDeviceLost(
      backend,
      information.reason ?? information.message ?? "Graphics device lost",
    );
  };
  let volumeTexture: THREE.Data3DTexture | undefined;
  try {
    volumeTexture =
      backend === "webgpu" ? await createVolumeDensityTexture() : undefined;
    return new ThreeWeatherRenderer(
      renderer,
      initialUpdate,
      backend,
      volumeTexture,
      onQualityChanged,
      appearanceOverrides,
    );
  } catch (error) {
    volumeTexture?.dispose();
    renderer.dispose();
    throw error;
  }
}
