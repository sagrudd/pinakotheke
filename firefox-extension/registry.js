// SPDX-License-Identifier: MPL-2.0
// Explicit adapter matching only; generic mode must be explicitly enabled.
export function canonicalOrigin(value) { return new URL(value).origin; }
export function matchAdapter(registry, url, experimental=false) { const target=new URL(url); return registry.adapters.find(adapter=>adapter.origins.includes(target.origin)&&!adapter.exclude_paths.some(path=>target.pathname.startsWith(path))&&(experimental||!adapter.experimental))||null; }
