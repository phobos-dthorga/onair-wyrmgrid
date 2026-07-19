import type {
  WeatherGraphicsPreferences,
  WeatherRenderingProfile,
} from "$lib/settings/types";

export type WeatherGraphicsPolicy = {
  profile: WeatherRenderingProfile;
  atmosphere: boolean;
  clouds: boolean;
  precipitation: boolean;
  lightning: boolean;
  dust: boolean;
  animation: boolean;
  lightningFlashes: boolean;
  particleScale: number;
  frameRate: number;
};

export function resolveWeatherGraphicsPolicy(
  preferences: WeatherGraphicsPreferences,
  lowResource: boolean,
  reducedMotion: boolean,
): WeatherGraphicsPolicy {
  const profile = lowResource
    ? "compatibility"
    : preferences.weather_rendering_profile;
  const atmosphere = profile !== "compatibility";
  const cinematic = profile === "cinematic";
  const animation = atmosphere && !reducedMotion;
  const lightning = atmosphere && preferences.weather_lightning_effects;

  return {
    profile,
    atmosphere,
    clouds: atmosphere && preferences.weather_cloud_effects,
    precipitation: atmosphere && preferences.weather_precipitation_effects,
    lightning,
    dust: atmosphere && preferences.weather_dust_effects,
    animation,
    lightningFlashes:
      cinematic &&
      animation &&
      lightning &&
      !preferences.reduce_weather_flashes,
    particleScale: cinematic ? 1 : atmosphere ? 0.58 : 0,
    frameRate: cinematic ? 30 : atmosphere ? 20 : 0,
  };
}

/** Two brief pulses in a long cycle; callers enforce the flash safety policy. */
export function lightningFlashOpacity(timeMs: number): number {
  const phase = ((timeMs % 6_400) + 6_400) % 6_400;
  if (phase < 80) return 0.62;
  if (phase >= 150 && phase < 215) return 0.34;
  return 0.025;
}
