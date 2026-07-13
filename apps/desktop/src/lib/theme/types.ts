export type ThemeColours = {
  canvas: string;
  surface: string;
  surface_elevated: string;
  surface_soft: string;
  text: string;
  text_muted: string;
  line: string;
  accent: string;
  highlight: string;
  danger: string;
  success: string;
  map_aircraft: string;
  map_fbo: string;
  map_label: string;
  map_halo: string;
};

export type ThemeManifest = {
  schema_version: number;
  id: string;
  name: string;
  author?: string;
  colors: ThemeColours;
  chart_palette: string[];
};

export type AvailableTheme = {
  manifest: ThemeManifest;
  built_in: boolean;
};

export type ThemeStatus = {
  selected_theme_id: string;
  active_theme: ThemeManifest;
  themes: AvailableTheme[];
};
