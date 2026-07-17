// SPDX-License-Identifier: MPL-2.0
const input = document.querySelector('#instance');
const instanceId = document.querySelector('#instance-id');
const pair = document.querySelector('#pair');
const origin = document.querySelector('#origin');
const status = document.querySelector('#status');
const diagnostic = document.querySelector('#diagnostic');
const sites = document.querySelector('#sites');
const images = document.querySelector('#images');
const videos = document.querySelector('#videos');
const xIngress = document.querySelector('#x-ingress');

const originPattern = value => {
  const parsed = new URL(value);
  if (parsed.protocol !== 'https:') throw new Error('HTTPS required');
  return `https://${parsed.hostname}/*`;
};

const serverRule = rule => ({
  origin: rule.origin,
  images: rule.media.includes('images'),
  videos: rule.media.includes('videos'),
  capture: rule.capture,
  substitution: rule.substitution,
  x_ingress: Boolean(rule.xIngress),
});

const localRule = rule => ({
  origin: rule.origin,
  capture: rule.capture,
  substitution: rule.substitution,
  xIngress: rule.x_ingress,
  media: [...(rule.images ? ['images'] : []), ...(rule.videos ? ['videos'] : [])],
});

async function persistSites(nextSites) {
  const stored = await browser.storage.local.get(['instanceUrl', 'pairId', 'siteCorpusRevision']);
  if (!stored.instanceUrl || !stored.pairId) {
    await browser.storage.local.set({ sites: nextSites });
    return { outcome: 'local' };
  }
  const response = await fetch(`${stored.instanceUrl}/products/pinakotheke/api/extension/v1/site-corpus`, {
    method: 'POST',
    cache: 'no-store',
    headers: { 'content-type': 'application/json', 'x-pinakotheke-pairing': stored.pairId },
    body: JSON.stringify({
      schema_version: 'pinakotheke.site-corpus.v1',
      expected_revision: stored.siteCorpusRevision || 0,
      rules: nextSites.map(serverRule),
    }),
  });
  if (response.status === 409) {
    const current = await response.json();
    await browser.storage.local.set({ sites: current.rules.map(localRule), siteCorpusRevision: current.revision });
    return { outcome: 'conflict', revision: current.revision };
  }
  if (!response.ok) throw new Error('Pinakotheke did not save the site corpus');
  const saved = await response.json();
  await browser.storage.local.set({ sites: saved.rules.map(localRule), siteCorpusRevision: saved.revision });
  return { outcome: 'saved', revision: saved.revision };
}

async function render() {
  const stored = await browser.storage.local.get(['instanceUrl', 'instanceId', 'pairId', 'sites', 'siteDiagnostics', 'siteCorpusRevision']);
  input.value = stored.instanceUrl || '';
  instanceId.value = stored.instanceId || '';
  pair.value = stored.pairId || '';
  diagnostic.textContent = stored.instanceUrl ? `Server site corpus revision ${stored.siteCorpusRevision || 0}.` : 'Site rules are local until Pinakotheke is paired.';
  sites.textContent = '';
  (stored.sites || []).forEach(rule => {
    const li = document.createElement('li');
    const last = stored.siteDiagnostics?.[rule.origin];
    li.textContent = `${rule.origin}: capture ${rule.capture ? 'on' : 'paused'}, substitution ${rule.substitution ? 'on' : 'paused'}, ${rule.media.join('/')}${rule.xIngress ? ', X ingress' : ''}${last ? `; ${last.state}. ${last.reason}.` : ''}`;
    for (const [label, key] of [['Toggle capture', 'capture'], ['Toggle substitution', 'substitution'], ['Remove', 'remove']]) {
      const button = document.createElement('button');
      button.textContent = label;
      button.onclick = async () => {
        const next = (stored.sites || []).map(item => ({ ...item, media: [...item.media] }));
        const selected = next.find(item => item.origin === rule.origin);
        const updated = key === 'remove' ? next.filter(item => item.origin !== rule.origin) : next;
        if (key !== 'remove') selected[key] = !selected[key];
        try {
          const result = await persistSites(updated);
          if (key === 'remove') await browser.permissions.remove({ origins: [originPattern(rule.origin)] });
          await browser.runtime.sendMessage({ command: 'sync-capture-observers' });
          status.textContent = result.outcome === 'conflict' ? 'Another device changed the site corpus. Its newer version has been restored; review it before trying again.' : 'Site corpus saved by Pinakotheke.';
        } catch (_) { status.textContent = 'Pinakotheke could not save this change; the previous corpus was retained.'; }
        render();
      };
      li.append(' ', button);
    }
    sites.append(li);
  });
}

document.querySelector('#save').onclick = async () => {
  let value;
  try { value = new URL(input.value.trim()).origin; } catch (_) { status.textContent = 'HTTPS URL, instance identifier, and pairing reference required.'; return; }
  if (!value.startsWith('https://') || !instanceId.value.trim() || !pair.value.trim()) { status.textContent = 'HTTPS URL, instance identifier, and pairing reference required.'; return; }
  const granted = await browser.permissions.request({ origins: [originPattern(value)] });
  if (!granted) { status.textContent = 'Permission for the Pinakotheke server was not granted.'; return; }
  status.textContent = 'Authenticating with Pinakotheke…';
  try {
    const response = await fetch(`${value}/products/pinakotheke/api/extension/v1/onboarding`, { cache: 'no-store', headers: { 'x-pinakotheke-pairing': pair.value.trim() } });
    if (!response.ok) throw new Error('authentication rejected');
    const setup = await response.json();
    if (setup.schema_version !== 'pinakotheke.extension-onboarding.v1' || setup.dasobjectstore_status !== 'Ready' || setup.instance_id !== instanceId.value.trim() || setup.pairing_reference !== pair.value.trim()) throw new Error('pairing payload mismatch');
    await browser.storage.local.set({ instanceUrl: value, instanceId: setup.instance_id, pairId: setup.pairing_reference, endpointId: setup.endpoint_id, objectStoreId: setup.object_store_id });
    const synchronized = await browser.runtime.sendMessage({ command: 'sync-site-corpus' });
    status.textContent = `Paired with ${setup.endpoint_id} · ${setup.object_store_id}. Site corpus ${synchronized.outcome}.`;
    render();
  } catch (_) { status.textContent = 'Pairing failed. Sign in to this Pinakotheke server, confirm its named ObjectStore is Ready, and use the payload shown by its web interface.'; }
};

document.querySelector('#enable').onclick = async () => {
  let value;
  try { value = new URL(origin.value).origin; } catch (_) { status.textContent = 'Enter an HTTPS origin.'; return; }
  if (!value.startsWith('https://') || (!images.checked && !videos.checked)) { status.textContent = 'Choose an HTTPS origin and at least one media class.'; return; }
  const granted = await browser.permissions.request({ origins: [originPattern(value)] });
  if (!granted) { status.textContent = 'Permission was not granted.'; return; }
  const stored = await browser.storage.local.get('sites');
  const rule = { origin: value, capture: true, substitution: false, xIngress: xIngress.checked, media: [...(images.checked ? ['images'] : []), ...(videos.checked ? ['videos'] : [])] };
  try {
    const result = await persistSites([...(stored.sites || []).filter(item => item.origin !== value), rule]);
    await browser.runtime.sendMessage({ command: 'sync-capture-observers' });
    status.textContent = result.outcome === 'conflict' ? 'Another device changed the site corpus. Its newer version has been restored.' : 'Enabled and saved to your Pinakotheke site corpus.';
  } catch (_) { status.textContent = 'Pinakotheke could not save this site; the previous corpus was retained.'; }
  render();
};

document.querySelector('#export-sites').onclick = async () => {
  const stored = await browser.storage.local.get(['sites', 'siteCorpusRevision']);
  const exportDocument = {
    schema_version: 'pinakotheke.site-corpus-export.v1',
    revision: stored.siteCorpusRevision || 0,
    rules: (stored.sites || []).map(serverRule),
  };
  const url = URL.createObjectURL(new Blob([`${JSON.stringify(exportDocument, null, 2)}\n`], { type: 'application/json' }));
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = 'pinakotheke-site-definitions.json';
  anchor.click();
  URL.revokeObjectURL(url);
  status.textContent = 'Site definitions exported without credentials.';
};

document.querySelector('#import-sites').onchange = async event => {
  const file = event.target.files?.[0];
  if (!file || file.size > 1024 * 1024) { status.textContent = 'Choose a site-definition JSON file smaller than 1 MiB.'; return; }
  try {
    const imported = JSON.parse(await file.text());
    if (imported.schema_version !== 'pinakotheke.site-corpus-export.v1' || !Array.isArray(imported.rules) || imported.rules.length > 256) throw new Error('invalid schema');
    const next = imported.rules.map(rule => {
      const exact = new URL(rule.origin);
      if (exact.protocol !== 'https:' || exact.origin !== rule.origin || exact.pathname !== '/' || exact.search || exact.hash || typeof rule.images !== 'boolean' || typeof rule.videos !== 'boolean' || (!rule.images && !rule.videos) || typeof rule.capture !== 'boolean' || typeof rule.substitution !== 'boolean' || typeof rule.x_ingress !== 'boolean') throw new Error('invalid rule');
      return localRule(rule);
    });
    if (new Set(next.map(rule => rule.origin)).size !== next.length) throw new Error('duplicate origin');
    const result = await persistSites(next);
    await browser.runtime.sendMessage({ command: 'sync-capture-observers' });
    status.textContent = result.outcome === 'conflict' ? 'Import conflicted with a newer server corpus; the server version was restored.' : 'Imported site definitions and saved them to Pinakotheke.';
    render();
  } catch (_) { status.textContent = 'The import was rejected; the existing site corpus was not changed.'; }
  event.target.value = '';
};

render();
