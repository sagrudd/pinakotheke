// SPDX-License-Identifier: MPL-2.0
const input=document.querySelector('#instance'), status=document.querySelector('#status');
browser.storage.local.get('instanceUrl').then(({instanceUrl})=>input.value=instanceUrl||'');
document.querySelector('#save').onclick=()=>{const value=input.value.trim(); if(value && !value.startsWith('https://')) {status.textContent='Use an HTTPS URL.'; return;} browser.storage.local.set({instanceUrl:value}).then(()=>status.textContent='Saved. No site permissions are enabled.');};
