// Lets `node --test` run the UI's plain TypeScript modules directly (Node strips the
// types itself). The source imports siblings without an extension ("./types"), as
// Vite allows, so try ".ts" when a relative import has none.
import { registerHooks } from "node:module";

registerHooks({
  resolve(specifier, context, next) {
    if (/^\.\.?\//.test(specifier) && !/\.[cm]?[jt]s$|\.json$/.test(specifier)) {
      try { return next(`${specifier}.ts`, context); } catch { /* fall through */ }
    }
    return next(specifier, context);
  },
});
