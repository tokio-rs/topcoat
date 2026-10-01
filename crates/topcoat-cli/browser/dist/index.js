"use strict";(()=>{var p=class{constructor(e,t){this.handlers=t;let i=new URL(e);i.protocol=i.protocol==="https:"?"wss:":"ws:",i.pathname="/ws",i.search="",i.hash="",this.url=i.href}handlers;url;lifetime=new AbortController;socket=null;retry;navigating=!1;get isNavigating(){return this.navigating}start(){let e={signal:this.lifetime.signal},t=()=>{this.navigating=!0,this.handlers.navigating()};window.navigation?.addEventListener("navigate",t,e),window.addEventListener("pagehide",t,e),window.navigation?.addEventListener("navigateerror",()=>{this.navigating=!1},e),window.addEventListener("pageshow",()=>{this.navigating=!1},e),this.connect(!1)}stop(){this.lifetime.abort(),clearTimeout(this.retry);let e=this.socket;this.socket=null,e?.close()}connect(e){let t=new WebSocket(this.url);this.socket=t,t.onopen=()=>{this.socket===t&&e&&!this.navigating&&this.handlers.reconnected()},t.onmessage=({data:i})=>{if(this.socket===t)switch(i){case"reload":case"rebuilding":case"build-failed":case"app-exited":case"up-to-date":this.handlers.event(i)}},t.onclose=()=>{this.socket===t&&(this.socket=null,this.reconnect())}}reconnect(){this.retry=setTimeout(()=>{this.lifetime.signal.aborted||(this.navigating?this.reconnect():this.connect(!0))},500)}};var x="topcoat:dev-runtime:v2";var b="application/x-ndjson";async function*T(n){let e=n.body;if(e===null){yield*g((await n.text()).split(`
`));return}let t=e.getReader(),i=new TextDecoder,r="";try{for(;;){let{done:o,value:s}=await t.read();r+=i.decode(s,{stream:!o});let l=r.split(`
`);if(r=l.pop()??"",yield*g(l),o)break}}finally{t.releaseLock()}yield*g([r])}function*g(n){for(let e of n)e.trim()!==""&&(yield JSON.parse(e))}function M(n,e,t,i,r={}){let o=Array.from(i),s=I(n,e,t),l=O(n,s,o,r);N(l,n,e?e.nextSibling:n.firstChild,t,o)}function I(n,e,t){let i=[],r=e?e.nextSibling:n.firstChild;for(;r&&r!==t;)i.push(r),r=r.nextSibling;return i}function O(n,e,t,i){let r=new Map;for(let a of e)for(let c of S(a))r.set(c.id,c);let o=new Set;for(let a of t)for(let c of S(a))r.has(c.id)&&o.add(c.id);let s=new Map,l=a=>{for(let c of a)for(let u of S(c)){if(!o.has(u.id))continue;let f=u;for(;f;){let h=s.get(f);if(h||(h=new Set,s.set(f,h)),h.add(u.id),f===c)break;f=f.parentNode}}};l(e),l(t);let d=new Map;for(let a of o){let c=r.get(a);c&&d.set(a,c)}return{options:i,preservedSelects:new WeakSet,persistent:o,idSets:s,oldById:d,active:n.ownerDocument?.activeElement??null}}function S(n){if(!(n instanceof Element))return[];let e=Array.from(n.querySelectorAll("[id]"));return n.id&&e.unshift(n),e}function N(n,e,t,i,r){let o=t;for(let s of r){if(e instanceof Element&&s instanceof Element){let l=e.closest("select");if(l&&n.preservedSelects.has(l)){let d=s instanceof HTMLOptionElement?[s]:Array.from(s.querySelectorAll("option"));for(let a of d)a.removeAttribute("selected"),a.selected=!1}}if(o&&o!==i){let l=P(n,s,o,i);if(l){F(o,l),o=y(n,l,s).nextSibling;continue}}if(s instanceof Element&&n.persistent.has(s.id)){let l=n.oldById.get(s.id);if(l){B(e,l,o),o=y(n,l,s).nextSibling;continue}}o=$(n,e,s,o).nextSibling}for(;o&&o!==i;){let s=o.nextSibling;o.remove(),o=s}}function P(n,e,t,i){let r=null,o=e.nextSibling,s=0,l=0,d=n.idSets.get(e)?.size??0,a=t;for(;a&&a!==i;){if(E(n,a,e)){if(U(n,a,e))return a;r===null&&!n.idSets.has(a)&&(r=a)}if(r===null&&o&&E(n,a,o)&&(s++,o=o.nextSibling,s>=2&&(r=void 0)),n.active&&a.contains(n.active)||(l+=n.idSets.get(a)?.size??0,l>d))break;a=a.nextSibling}return r??null}function E(n,e,t){return!(e.nodeType!==t.nodeType||e instanceof Element&&t instanceof Element&&(e.tagName!==t.tagName||e.id&&e.id!==t.id&&n.persistent.has(e.id)))}function U(n,e,t){let i=n.idSets.get(e),r=n.idSets.get(t);if(!i||!r)return!1;for(let o of i)if(r.has(o))return!0;return!1}function F(n,e){let t=n;for(;t&&t!==e;){let i=t.nextSibling;t.remove(),t=i}}function y(n,e,t){if(e instanceof Element&&t instanceof Element){if(e.tagName===t.tagName){let i=_(n,e);return q(e,t,i),n.options.preserveFormState&&e instanceof HTMLTextAreaElement||N(n,e,e.firstChild,null,Array.from(t.childNodes)),W(e,t,i),e}}else if(e.nodeType===t.nodeType)return e.nodeValue!==t.nodeValue&&(e.nodeValue=t.nodeValue),e;return e.replaceWith(t),t}function $(n,e,t,i){if(t instanceof Element&&n.idSets.has(t)){let o=(e.ownerDocument??document).createElementNS(t.namespaceURI,t.localName);return e.insertBefore(o,i),y(n,o,t)}return e.insertBefore(t,i),t}function B(n,e,t){let i=n;if(i.moveBefore&&e.isConnected)try{i.moveBefore(e,t);return}catch{}n.insertBefore(e,t)}function _(n,e){let t=new Set;if(e===n.active&&t.add("value"),!n.options.preserveFormState)return t;if(e instanceof HTMLInputElement)t.add("value"),t.add("checked");else if(e instanceof HTMLTextAreaElement)t.add("value");else if(e instanceof HTMLSelectElement)n.preservedSelects.add(e),t.add("value");else if(e instanceof HTMLOptionElement){let i=e.closest("select");i&&n.preservedSelects.has(i)&&t.add("selected")}return t}function q(n,e,t){for(let i of Array.from(e.attributes))t.has(i.name)||(i.namespaceURI===null?n.getAttribute(i.name)!==i.value&&n.setAttribute(i.name,i.value):n.getAttributeNS(i.namespaceURI,i.localName)!==i.value&&n.setAttributeNS(i.namespaceURI,i.name,i.value));for(let i of Array.from(n.attributes))t.has(i.name)||(i.namespaceURI===null?e.hasAttribute(i.name)||n.removeAttribute(i.name):e.hasAttributeNS(i.namespaceURI,i.localName)||n.removeAttributeNS(i.namespaceURI,i.localName))}function W(n,e,t){n instanceof HTMLInputElement&&e instanceof HTMLInputElement?(!t.has("checked")&&n.checked!==e.checked&&(n.checked=e.checked),!t.has("value")&&n.value!==e.value&&(n.value=e.value)):n instanceof HTMLTextAreaElement&&e instanceof HTMLTextAreaElement?!t.has("value")&&n.value!==e.value&&(n.value=e.value):n instanceof HTMLOptionElement&&e instanceof HTMLOptionElement?!t.has("selected")&&n.selected!==e.selected&&(n.selected=e.selected):n instanceof HTMLSelectElement&&e instanceof HTMLSelectElement&&!t.has("value")&&n.value!==e.value&&(n.value=e.value)}function w(n){let e=i=>JSON.stringify(Array.from(i.scripts,r=>r.outerHTML)),t=i=>{let r=i.doctype;return JSON.stringify(r&&[r.name,r.publicId,r.systemId])};return e(document)===e(n)&&t(document)===t(n)&&document.querySelector("base")?.outerHTML===n.querySelector("base")?.outerHTML}function C(n){let e=document.documentElement,t=n.documentElement,{scrollX:i,scrollY:r}=window;for(let o of Array.from(e.attributes))t.hasAttribute(o.name)||e.removeAttribute(o.name);for(let o of Array.from(t.attributes))e.setAttribute(o.name,o.value);M(e,null,null,t.childNodes,{preserveFormState:!0}),window.scrollTo({left:i,top:r,behavior:"instant"})}function k(n,e){let t=document.createTreeWalker(document,NodeFilter.SHOW_COMMENT),i=null;for(let r=t.nextNode();r;r=t.nextNode()){let o=r;if(o.data.trim()===`::topcoat::region::start(${n})`)i=o;else if(o.data.trim()===`::topcoat::region::end(${n})`){let s=i?.parentNode;if(!(s instanceof Element)||o.parentNode!==s)return;let l=document.createElementNS(s.namespaceURI,s.localName);l.innerHTML=e,M(s,i,o,l.childNodes,{preserveFormState:!0});return}}}var m=class{constructor(e,t,i){this.navigating=e;this.beforeUpdate=t;this.reportError=i}navigating;beforeUpdate;reportError;controller=null;cancel(){this.controller?.abort(),this.controller=null}async refresh(){if(this.cancel(),this.navigating())return;if(document.readyState==="loading"){location.reload();return}let e=new AbortController;this.controller=e;let t,i=new Promise(s=>{t=s});e.signal.addEventListener("abort",t,{once:!0});let r=location.href,o=()=>this.controller===e&&!this.navigating()&&location.href===r;try{let s={};window.dispatchEvent(new CustomEvent(x,{detail:s}));let l=await(s.runtime?.request(e.signal,i)??fetch(r,{cache:"no-store",headers:{Accept:b},signal:e.signal}));if(!o())return;if(l.redirected){location.assign(l.url);return}if(!l.ok)throw new Error(`Refresh failed: ${l.status} ${l.statusText}`);if(l.headers.get("Content-Type")?.split(";")[0]?.trim().toLowerCase()!==b){location.reload();return}let d=!1;for await(let a of T(l)){if(!o())return;switch(a.t){case"snapshot":{let c=new DOMParser().parseFromString(a.html,"text/html");if(!w(c)){location.reload();return}this.beforeUpdate();let u=()=>C(c);s.runtime?s.runtime.replace(u):u(),d=!0;break}case"swap":if(!d)throw new Error("Refresh update arrived before its page");s.runtime?s.runtime.swap(a.region,a.html):k(a.region,a.html);break;case"redirect":location.assign(a.location);return;case"error":throw new Error(`Refresh render failed: ${a.status}`)}}if(o()&&!d)throw new Error("Refresh response contained no page")}catch(s){o()&&this.reportError(s)}finally{e.abort(),t(),this.controller===e&&(this.controller=null)}}};var v=class{constructor(e){this.enabled=e;if(e){document.addEventListener("DOMContentLoaded",()=>this.render(),{once:!0});for(let t of["400","600"]){let i=new FontFace(H,`url(${V}${t}-normal.woff2) format("woff2")`,{weight:t,display:"swap"});document.fonts.add(i),i.load().catch(()=>{})}}}enabled;pill=null;current=null;show(e,t=!1){this.enabled&&(this.current={label:e,isError:t},this.render())}hide(){this.current=null,this.pill?.host.remove()}render(){if(!this.current||!document.body)return;this.pill??=this.createPill();let{host:e,label:t,spinner:i}=this.pill;t.textContent=this.current.label,t.className=this.current.isError?"error":"busy",i.style.display=this.current.isError?"none":"",e.isConnected||document.body.append(e)}createPill(){let e=document.createElement("topcoat-dev-status");e.style.cssText=Y;let t=e.attachShadow({mode:"open"});t.innerHTML=J;let i=t.querySelector("b"),r=t.querySelector(".spinner");return t.querySelector("button").addEventListener("click",()=>this.hide()),{host:e,label:i,spinner:r}}},L="#a1a1aa",A="#fca5a5",H="Topcoat Dev",V="https://cdn.jsdelivr.net/fontsource/fonts/lexend-deca@latest/latin-",R=n=>`<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">${n}</svg>`,j=R('<path d="M18 6 6 18"/><path d="m6 6 12 12"/>'),z=R('<path d="M21 12a9 9 0 1 1-6.219-8.56"/>'),Y=`
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
  font: 12px/1 "${H}", ui-sans-serif, system-ui, sans-serif;
  -webkit-font-smoothing: antialiased;
  user-select: none;
`,J=`
  <style>
    .brand {
      color: ${L};
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
      color: ${L};
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
        linear-gradient(100deg, ${A} 30%, #fecaca 50%, ${A} 70%);
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
  <span class="spinner">${z}</span>
  <button class="dismiss" aria-label="Dismiss">${j}</button>
`;function G(n){let e=new v(n.dataset.statusIndicator!=="false"),t=new p(n.src,{event:o=>r[o](),reconnected:()=>{i.refresh()},navigating:()=>i.cancel()}),i=new m(()=>t.isNavigating,()=>e.hide(),o=>{console.error("[topcoat dev]",o),e.show("refresh failed",!0)}),r={reload:()=>{e.hide(),i.refresh()},rebuilding:()=>{i.cancel(),e.show("rebuilding")},"build-failed":()=>e.show("build failed",!0),"app-exited":()=>e.show("app exited",!0),"up-to-date":()=>e.hide()};t.start()}var D=document.currentScript;D instanceof HTMLScriptElement&&G(D);})();
