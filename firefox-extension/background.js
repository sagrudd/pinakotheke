// SPDX-License-Identifier: MPL-2.0
// No host permissions, cookie APIs, webRequest APIs, or capture behavior exist here.
browser.runtime.onInstalled.addListener(() => browser.storage.local.set({ instanceUrl: "" }));
