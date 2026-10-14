"use strict";(()=>{var h=class{constructor(e,t){this.handlers=t;let n=new URL(e);n.protocol=n.protocol==="https:"?"wss:":"ws:",n.pathname="/ws",n.search="",n.hash="",this.url=n.href}handlers;url;lifetime=new AbortController;socket=null;retry;navigating=!1;get isNavigating(){return this.navigating}start(){let e={signal:this.lifetime.signal},t=()=>{this.navigating=!0,this.handlers.navigating()};window.navigation?.addEventListener("navigate",t,e),window.addEventListener("pagehide",t,e),window.navigation?.addEventListener("navigateerror",()=>{this.navigating=!1},e),window.addEventListener("pageshow",()=>{this.navigating=!1},e),this.connect(!1)}stop(){this.lifetime.abort(),clearTimeout(this.retry);let e=this.socket;this.socket=null,e?.close()}connect(e){let t=new WebSocket(this.url);this.socket=t,t.onopen=()=>{this.socket===t&&e&&!this.navigating&&this.handlers.reconnected()},t.onmessage=({data:n})=>{if(this.socket===t)switch(n){case"reload":case"rebuilding":case"build-failed":case"app-exited":case"up-to-date":this.handlers.event(n)}},t.onclose=()=>{this.socket===t&&(this.socket=null,this.reconnect())}}reconnect(){this.retry=setTimeout(()=>{this.lifetime.signal.aborted||(this.navigating?this.reconnect():this.connect(!0))},500)}};var w="topcoat:dev-runtime:v2";var b="application/x-ndjson";async function*x(o){let e=o.body;if(e===null){yield*v((await o.text()).split(`
`));return}let t=e.getReader(),n=new TextDecoder,i="";try{for(;;){let{done:r,value:s}=await t.read();i+=n.decode(s,{stream:!r});let l=i.split(`
`);if(i=l.pop()??"",yield*v(l),r)break}}finally{t.releaseLock()}yield*v([i])}function*v(o){for(let e of o)e.trim()!==""&&(yield JSON.parse(e))}function E(o,e,t,n,i={}){let r=Array.from(n),s=H(o,e,t),l=O(o,s,r,i);C(l,o,e?e.nextSibling:o.firstChild,t,r)}function H(o,e,t){let n=[],i=e?e.nextSibling:o.firstChild;for(;i&&i!==t;)n.push(i),i=i.nextSibling;return n}function O(o,e,t,n){let i=new Map;for(let a of e)for(let c of y(a))i.set(c.id,c);let r=new Set;for(let a of t)for(let c of y(a))i.has(c.id)&&r.add(c.id);let s=new Map,l=a=>{for(let c of a)for(let u of y(c)){if(!r.has(u.id))continue;let f=u;for(;f;){let p=s.get(f);if(p||(p=new Set,s.set(f,p)),p.add(u.id),f===c)break;f=f.parentNode}}};l(e),l(t);let d=new Map;for(let a of r){let c=i.get(a);c&&d.set(a,c)}return{options:n,preservedSelects:new WeakSet,persistent:r,idSets:s,oldById:d,active:o.ownerDocument?.activeElement??null}}function y(o){if(!(o instanceof Element))return[];let e=Array.from(o.querySelectorAll("[id]"));return o.id&&e.unshift(o),e}function C(o,e,t,n,i){let r=t;for(let s of i){if(e instanceof Element&&s instanceof Element){let l=e.closest("select");if(l&&o.preservedSelects.has(l)){let d=s instanceof HTMLOptionElement?[s]:Array.from(s.querySelectorAll("option"));for(let a of d)a.removeAttribute("selected"),a.selected=!1}}if(r&&r!==n){let l=P(o,s,r,n);if(l){F(r,l),r=S(o,l,s).nextSibling;continue}}if(s instanceof Element&&o.persistent.has(s.id)){let l=o.oldById.get(s.id);if(l){B(e,l,r),r=S(o,l,s).nextSibling;continue}}r=$(o,e,s,r).nextSibling}for(;r&&r!==n;){let s=r.nextSibling;r.remove(),r=s}}function P(o,e,t,n){let i=null,r=e.nextSibling,s=0,l=0,d=o.idSets.get(e)?.size??0,a=t;for(;a&&a!==n;){if(N(o,a,e)){if(U(o,a,e))return a;i===null&&!o.idSets.has(a)&&(i=a)}if(i===null&&r&&N(o,a,r)&&(s++,r=r.nextSibling,s>=2&&(i=void 0)),o.active&&a.contains(o.active)||(l+=o.idSets.get(a)?.size??0,l>d))break;a=a.nextSibling}return i??null}function N(o,e,t){return!(e.nodeType!==t.nodeType||e instanceof Element&&t instanceof Element&&(e.tagName!==t.tagName||e.id&&e.id!==t.id&&o.persistent.has(e.id)))}function U(o,e,t){let n=o.idSets.get(e),i=o.idSets.get(t);if(!n||!i)return!1;for(let r of n)if(i.has(r))return!0;return!1}function F(o,e){let t=o;for(;t&&t!==e;){let n=t.nextSibling;t.remove(),t=n}}function S(o,e,t){if(e instanceof Element&&t instanceof Element){if(e.tagName===t.tagName)return q(e,t,_(o,e)),o.options.preserveFormState&&e instanceof HTMLTextAreaElement||C(o,e,e.firstChild,null,Array.from(t.childNodes)),e}else if(e.nodeType===t.nodeType)return e.nodeValue!==t.nodeValue&&(e.nodeValue=t.nodeValue),e;return e.replaceWith(t),t}function $(o,e,t,n){if(t instanceof Element&&o.idSets.has(t)){let r=(e.ownerDocument??document).createElementNS(t.namespaceURI,t.localName);return e.insertBefore(r,n),S(o,r,t)}return e.insertBefore(t,n),t}function B(o,e,t){let n=o;if(n.moveBefore&&e.isConnected)try{n.moveBefore(e,t);return}catch{}o.insertBefore(e,t)}function _(o,e){let t=new Set;if(!o.options.preserveFormState)return t;if(e instanceof HTMLInputElement)t.add("value"),t.add("checked");else if(e instanceof HTMLTextAreaElement)t.add("value");else if(e instanceof HTMLSelectElement)o.preservedSelects.add(e),t.add("value");else if(e instanceof HTMLOptionElement){let n=e.closest("select");n&&o.preservedSelects.has(n)&&t.add("selected")}return t}function q(o,e,t){for(let n of Array.from(e.attributes))t.has(n.name)||(n.namespaceURI===null?o.getAttribute(n.name)!==n.value&&o.setAttribute(n.name,n.value):o.getAttributeNS(n.namespaceURI,n.localName)!==n.value&&o.setAttributeNS(n.namespaceURI,n.name,n.value));for(let n of Array.from(o.attributes))t.has(n.name)||(n.namespaceURI===null?e.hasAttribute(n.name)||o.removeAttribute(n.name):e.hasAttributeNS(n.namespaceURI,n.localName)||o.removeAttributeNS(n.namespaceURI,n.localName))}function M(o){let e=n=>JSON.stringify(Array.from(n.scripts,i=>i.outerHTML)),t=n=>{let i=n.doctype;return JSON.stringify(i&&[i.name,i.publicId,i.systemId])};return e(document)===e(o)&&t(document)===t(o)&&document.querySelector("base")?.outerHTML===o.querySelector("base")?.outerHTML}function T(o){let e=document.documentElement,t=o.documentElement,{scrollX:n,scrollY:i}=window;for(let r of Array.from(e.attributes))t.hasAttribute(r.name)||e.removeAttribute(r.name);for(let r of Array.from(t.attributes))e.setAttribute(r.name,r.value);E(e,null,null,t.childNodes,{preserveFormState:!0}),window.scrollTo({left:n,top:i,behavior:"instant"})}function k(o,e){let t=document.createTreeWalker(document,NodeFilter.SHOW_COMMENT),n=null;for(let i=t.nextNode();i;i=t.nextNode()){let r=i;if(r.data.trim()===`::topcoat::region::start(${o})`)n=r;else if(r.data.trim()===`::topcoat::region::end(${o})`){let s=n?.parentNode;if(!(s instanceof Element)||r.parentNode!==s)return;let l=document.createElementNS(s.namespaceURI,s.localName);l.innerHTML=e,E(s,n,r,l.childNodes,{preserveFormState:!0});return}}}var m=class{constructor(e,t,n){this.navigating=e;this.beforeUpdate=t;this.reportError=n}navigating;beforeUpdate;reportError;controller=null;cancel(){this.controller?.abort(),this.controller=null}async refresh(){if(this.cancel(),this.navigating())return;if(document.readyState==="loading"){location.reload();return}let e=new AbortController;this.controller=e;let t,n=new Promise(s=>{t=s});e.signal.addEventListener("abort",t,{once:!0});let i=location.href,r=()=>this.controller===e&&!this.navigating()&&location.href===i;try{let s={};window.dispatchEvent(new CustomEvent(w,{detail:s}));let l=await(s.runtime?.request(e.signal,n)??fetch(i,{cache:"no-store",headers:{Accept:b},signal:e.signal}));if(!r())return;if(l.redirected){location.assign(l.url);return}if(!l.ok)throw new Error(`Refresh failed: ${l.status} ${l.statusText}`);if(l.headers.get("Content-Type")?.split(";")[0]?.trim().toLowerCase()!==b){location.reload();return}let d=!1;for await(let a of x(l)){if(!r())return;switch(a.t){case"snapshot":{let c=new DOMParser().parseFromString(a.html,"text/html");if(!M(c)){location.reload();return}this.beforeUpdate();let u=()=>T(c);s.runtime?s.runtime.replace(u):u(),d=!0;break}case"swap":if(!d)throw new Error("Refresh update arrived before its page");s.runtime?s.runtime.swap(a.region,a.html):k(a.region,a.html);break;case"redirect":location.assign(a.location);return;case"error":throw new Error(`Refresh render failed: ${a.status}`)}}if(r()&&!d)throw new Error("Refresh response contained no page")}catch(s){r()&&this.reportError(s)}finally{e.abort(),t(),this.controller===e&&(this.controller=null)}}};var g=class{constructor(e){this.enabled=e;if(e){document.addEventListener("DOMContentLoaded",()=>this.render(),{once:!0});for(let t of["400","600"]){let n=new FontFace(R,`url(${W}${t}-normal.woff2) format("woff2")`,{weight:t,display:"swap"});document.fonts.add(n),n.load().catch(()=>{})}}}enabled;pill=null;current=null;show(e,t=!1){this.enabled&&(this.current={label:e,isError:t},this.render())}hide(){this.current=null,this.pill?.host.remove()}render(){if(!this.current||!document.body)return;this.pill??=this.createPill();let{host:e,label:t,spinner:n}=this.pill;t.textContent=this.current.label,t.className=this.current.isError?"error":"busy",n.style.display=this.current.isError?"none":"",e.isConnected||document.body.append(e)}createPill(){let e=document.createElement("topcoat-dev-status");e.style.cssText=z;let t=e.attachShadow({mode:"open"});t.innerHTML=Y;let n=t.querySelector("b"),i=t.querySelector(".spinner");return t.querySelector("button").addEventListener("click",()=>this.hide()),{host:e,label:n,spinner:i}}},A="#a1a1aa",L="#fca5a5",R="Topcoat Dev",W="https://cdn.jsdelivr.net/fontsource/fonts/lexend-deca@latest/latin-",D=o=>`<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">${o}</svg>`,V=D('<path d="M18 6 6 18"/><path d="m6 6 12 12"/>'),j=D('<path d="M21 12a9 9 0 1 1-6.219-8.56"/>'),z=`
  all: initial;
  position: fixed;
  bottom: 16px;
  left: 16px;
  z-index: 2147483647;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 7px 8px 7px 14px;
  background: #0a0a0a;
  color: #fff;
  border: 1px solid #000;
  border-radius: 8px;
  font: 12px/1 "${R}", ui-sans-serif, system-ui, sans-serif;
  -webkit-font-smoothing: antialiased;
  user-select: none;
`,Y=`
  <style>
    .brand {
      color: ${A};
    }
    b {
      font-weight: 600;
    }

    /* all:unset strips the UA button styles. */
    .dismiss {
      all: unset;
      display: flex;
      align-items: center;
      justify-content: center;
      width: 18px;
      height: 18px;
      border-radius: 4px;
      cursor: pointer;
      color: ${A};
      transition: color 0.15s ease;
    }
    .dismiss:hover {
      color: #fff;
    }

    .busy {
      color: #a5f3fc;
    }
    .spinner {
      display: flex;
      color: #a5f3fc;
      animation: spin 1s linear infinite;
    }
    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    /* Error labels catch the eye with a soft highlight sweeping across
       the text once every three seconds. */
    .error {
      background-image:
        linear-gradient(100deg, ${L} 30%, #fecaca 50%, ${L} 70%);
      background-size: 300% 100%;
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
      animation: shimmer 3s ease-in-out infinite;
    }
    @keyframes shimmer {
      0% { background-position: 100% 0; }
      25% { background-position: 0 0; }
      100% { background-position: 0 0; }
    }
  </style>
  <span class="brand">topcoat</span>
  <b></b>
  <span class="spinner">${j}</span>
  <button class="dismiss" aria-label="Dismiss">${V}</button>
`;function J(o){let e=new g(o.dataset.statusIndicator!=="false"),t=new h(o.src,{event:r=>i[r](),reconnected:()=>{n.refresh()},navigating:()=>n.cancel()}),n=new m(()=>t.isNavigating,()=>e.hide(),r=>{console.error("[topcoat dev]",r),e.show("refresh failed",!0)}),i={reload:()=>{e.hide(),n.refresh()},rebuilding:()=>{n.cancel(),e.show("rebuilding")},"build-failed":()=>e.show("build failed",!0),"app-exited":()=>e.show("app exited",!0),"up-to-date":()=>e.hide()};t.start()}var I=document.currentScript;I instanceof HTMLScriptElement&&J(I);})();
