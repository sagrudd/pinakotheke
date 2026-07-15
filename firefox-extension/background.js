// SPDX-License-Identifier: MPL-2.0
// No cookies, webRequest interception, credentials, or automatic navigation.
browser.runtime.onInstalled.addListener(() => browser.storage.local.set({ instanceUrl: "", pairId: "", sites: [] }));
browser.action.onClicked.addListener(async tab => {
  const {instanceUrl,pairId,sites=[]}=await browser.storage.local.get(["instanceUrl","pairId","sites"]);
  const origin=tab.url&&new URL(tab.url).origin, rule=sites.find(site=>site.origin===origin); if(!origin||!rule?.capture||!instanceUrl||!pairId) return;
  const [{result=[]}]=await browser.scripting.executeScript({target:{tabId:tab.id},func:()=>[...document.images].filter(image=>image.complete&&image.currentSrc).map(image=>({url:image.currentSrc,width:image.naturalWidth,height:image.naturalHeight})).slice(0,64)});
  await fetch(`${instanceUrl}/api/extension/v1/observed-media`,{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify({pair_id:pairId,origin,observed:result})}).catch(()=>undefined);
});
