import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    // Labels here are visual captions next to controls, not wrapped form labels.
    warningFilter: (w) => w.code !== "a11y_label_has_associated_control",
  },
};
