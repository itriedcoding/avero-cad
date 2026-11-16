export const MAP_SIZE = 5355;

export interface MapOption {
  label: string;
  url: string;
}

/** Official ER:LC map images, served from api.erlc.gg/maps. */
export const MAP_OPTIONS: MapOption[] = [
  { label: "Summer (postal codes)", url: "https://api.erlc.gg/maps/summer_postals.png" },
  { label: "Summer (blank)", url: "https://api.erlc.gg/maps/summer_blank.png" },
  { label: "Spring (postal codes)", url: "https://api.erlc.gg/maps/spring_postals.png" },
  { label: "Spring (blank)", url: "https://api.erlc.gg/maps/spring_blank.png" },
  { label: "Fall (postal codes)", url: "https://api.erlc.gg/maps/fall_postals.png" },
  { label: "Fall (blank)", url: "https://api.erlc.gg/maps/fall_blank.png" },
  { label: "Winter (postal codes)", url: "https://api.erlc.gg/maps/snow_postals.png" },
  { label: "Winter (blank)", url: "https://api.erlc.gg/maps/snow_blank.png" },
];
