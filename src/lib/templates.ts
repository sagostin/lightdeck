// Setup templates: ready-made scene layouts made of building blocks (blocks.ts), built
// from the rig with fixed rules, no AI. Open one in the scene builder to change it.
import type { Plan } from "./design";
import type { Genre } from "./genres";
import type { Show } from "./types";
import { analyseRig, buildScenes, type BlockUse, type SceneSpec } from "./blocks";

export { analyseRig } from "./blocks";

export interface Template { id: string; name: string; description: string }

export const TEMPLATES: Template[] = [
  { id: "club", name: "Club night", description: "Warm-up, Build, Drop and Breakdown scenes: slow waves, energy builds, kick-driven colour with strobe and fog on the drop." },
  { id: "party", name: "Party", description: "Groove and Peak scenes: colourful palette steps, sparkles and node hops, strobe and fog at the peak." },
  { id: "chill", name: "Chill / lounge", description: "Glow and Drift scenes: long colour fades and slow waves. No strobe, no fog." },
  { id: "test", name: "Test my rig", description: "One scene where every group flashes on the kick and changes colour each bar, and lasers change look. For checking patching and detection." },
];

const b = (block: string, speed: BlockUse["speed"], more: Partial<BlockUse> = {}): BlockUse => ({ block, speed, target: "all", ...more });
const laser = (sound: string, tag: string, speed: BlockUse["speed"]) => b("laser", speed, { target: "lasers", sound, tag });
const fog = () => b("fog", "medium", { target: "fog" });

/** A template's scenes, as you'd see them in the scene builder. */
export function templateScenes(id: string, show: Show): SceneSpec[] {
  const rig = analyseRig(show);
  switch (id) {
    case "club":
      return [
        { name: "Warm-up", sections: ["breakdown", "groove"], blocks: [b("wave", "slow", { palette: "calm" }), b("sparkle", "medium", { target: "b", sound: "hat", palette: "hot" }), laser("bar", "calm", "slow")] },
        { name: "Build", sections: ["build"], blocks: [b("colour_bar", "medium", { sound: "bar", palette: "hot" }), b("follow", "medium", { sound: "energy" }), laser("kick", "", "medium")] },
        { name: "Drop", sections: ["drop"], blocks: [b("colour_hits", "fast", { sound: "kick", palette: "hot" }), b("strobe", "medium", { target: "a", sound: "snare" }), laser("kick", "peak", "fast"), fog()] },
        { name: "Breakdown", sections: ["breakdown"], blocks: [b("follow", "medium", { sound: "vocals" }), b("wave", "medium", { palette: "calm" }), laser("bar", "calm", "slow")] },
      ];
    case "party":
      return [
        { name: "Groove", sections: ["breakdown", "groove", "build"], blocks: [b("colour_hits", "medium", { sound: "kick", palette: "hot" }), b("sparkle", "fast", { target: "b", sound: "hat", palette: "Rainbow" }), laser("bar", "", "medium")] },
        { name: "Peak", sections: ["drop"], blocks: [b("wave", "fast", { palette: "Rainbow" }), b("node_hop", "medium", { target: "a", sound: "kick", palette: "hot" }), b("strobe", "slow", { sound: "snare" }), laser("kick", "peak", "fast"), fog()] },
      ];
    case "chill":
      return [
        { name: "Glow", sections: ["breakdown"], blocks: [b("wave", "slow", { palette: "calm" }), b("sparkle", "slow", { target: "b", sound: "hat", palette: "hot" }), laser("bar", "calm", "slow")] },
        { name: "Drift", sections: ["groove", "build", "drop"], blocks: [b("colour_bar", "slow", { sound: "bar", palette: "calm" }), b("follow", "slow", { sound: "level" }), laser("bar", "calm", "slow")] },
      ];
    case "test": {
      const halves = rig.glow.length >= 2 && JSON.stringify(rig.a) !== JSON.stringify(rig.b);
      const flashes = halves
        ? [b("flash", "medium", { target: "a", sound: "kick" }), b("flash", "medium", { target: "b", sound: "kick" })]
        : [b("flash", "medium", { sound: "kick" })];
      return [{ name: "Test", sections: [], blocks: [...flashes, b("colour_bar", "fast", { sound: "bar", palette: "hot" }), laser("kick", "", "fast")] }];
    }
  }
  return [];
}

export function buildTemplate(show: Show, id: string, genre: Genre | null): Plan {
  let scenes = templateScenes(id, show);
  // Ambient music gets no strobe, whatever the template.
  if (genre?.id === "ambient") scenes = scenes.map((s) => ({ ...s, blocks: s.blocks.filter((x) => x.block !== "strobe") }));
  const t = TEMPLATES.find((x) => x.id === id)!;
  return buildScenes(show, scenes, {
    genre, title: `${t.name}${genre ? ` · ${genre.name}` : ""}`,
    moods: id === "chill" && !genre ? ["Sunset", "Forest"] : undefined,
  }).plan;
}
