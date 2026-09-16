"use strict";(()=>{var h=class{constructor(e,t){this.handlers=t;let i=new URL(e);i.protocol=i.protocol==="https:"?"wss:":"ws:",i.pathname="/ws",i.search="",i.hash="",this.url=i.href}handlers;url;lifetime=new AbortController;socket=null;retry;navigating=!1;get isNavigating(){return this.navigating}start(){let e={signal:this.lifetime.signal},t=()=>{this.navigating=!0,this.handlers.navigating()};window.navigation?.addEventListener("navigate",t,e),window.addEventListener("pagehide",t,e),window.navigation?.addEventListener("navigateerror",()=>{this.navigating=!1},e),window.addEventListener("pageshow",()=>{this.navigating=!1},e),this.connect(!1)}stop(){this.lifetime.abort(),clearTimeout(this.retry);let e=this.socket;this.socket=null,e?.close()}connect(e){let t=new WebSocket(this.url);this.socket=t,t.onopen=()=>{this.socket===t&&e&&!this.navigating&&this.handlers.reconnected()},t.onmessage=({data:i})=>{if(this.socket===t)switch(i){case"reload":case"rebuilding":case"build-failed":case"app-exited":case"up-to-date":this.handlers.event(i)}},t.onclose=()=>{this.socket===t&&(this.socket=null,this.reconnect())}}reconnect(){this.retry=setTimeout(()=>{this.lifetime.signal.aborted||(this.navigating?this.reconnect():this.connect(!0))},500)}};var S="topcoat:dev-runtime:v1";function x(n,e,t,i,r={}){let o=Array.from(i),s=A(n,e,t),l=H(n,s,o,r);C(l,n,e?e.nextSibling:n.firstChild,t,o)}function A(n,e,t){let i=[],r=e?e.nextSibling:n.firstChild;for(;r&&r!==t;)i.push(r),r=r.nextSibling;return i}function H(n,e,t,i){let r=new Map;for(let a of e)for(let c of g(a))r.set(c.id,c);let o=new Set;for(let a of t)for(let c of g(a))r.has(c.id)&&o.add(c.id);let s=new Map,l=a=>{for(let c of a)for(let v of g(c)){if(!o.has(v.id))continue;let u=v;for(;u;){let f=s.get(u);if(f||(f=new Set,s.set(u,f)),f.add(v.id),u===c)break;u=u.parentNode}}};l(e),l(t);let d=new Map;for(let a of o){let c=r.get(a);c&&d.set(a,c)}return{options:i,preservedSelects:new WeakSet,persistent:o,idSets:s,oldById:d,active:n.ownerDocument?.activeElement??null}}function g(n){if(!(n instanceof Element))return[];let e=Array.from(n.querySelectorAll("[id]"));return n.id&&e.unshift(n),e}function C(n,e,t,i,r){let o=t;for(let s of r){if(e instanceof Element&&s instanceof Element){let l=e.closest("select");if(l&&n.preservedSelects.has(l)){let d=s instanceof HTMLOptionElement?[s]:Array.from(s.querySelectorAll("option"));for(let a of d)a.removeAttribute("selected"),a.selected=!1}}if(o&&o!==i){let l=D(n,s,o,i);if(l){R(o,l),o=b(n,l,s).nextSibling;continue}}if(s instanceof Element&&n.persistent.has(s.id)){let l=n.oldById.get(s.id);if(l){P(e,l,o),o=b(n,l,s).nextSibling;continue}}o=O(n,e,s,o).nextSibling}for(;o&&o!==i;){let s=o.nextSibling;o.remove(),o=s}}function D(n,e,t,i){let r=null,o=e.nextSibling,s=0,l=0,d=n.idSets.get(e)?.size??0,a=t;for(;a&&a!==i;){if(y(n,a,e)){if(I(n,a,e))return a;r===null&&!n.idSets.has(a)&&(r=a)}if(r===null&&o&&y(n,a,o)&&(s++,o=o.nextSibling,s>=2&&(r=void 0)),n.active&&a.contains(n.active)||(l+=n.idSets.get(a)?.size??0,l>d))break;a=a.nextSibling}return r??null}function y(n,e,t){return!(e.nodeType!==t.nodeType||e instanceof Element&&t instanceof Element&&(e.tagName!==t.tagName||e.id&&e.id!==t.id&&n.persistent.has(e.id)))}function I(n,e,t){let i=n.idSets.get(e),r=n.idSets.get(t);if(!i||!r)return!1;for(let o of i)if(r.has(o))return!0;return!1}function R(n,e){let t=n;for(;t&&t!==e;){let i=t.nextSibling;t.remove(),t=i}}function b(n,e,t){if(e instanceof Element&&t instanceof Element){if(e.tagName===t.tagName){let i=U(n,e);return B(e,t,i),n.options.preserveFormState&&e instanceof HTMLTextAreaElement||C(n,e,e.firstChild,null,Array.from(t.childNodes)),$(e,t,i),e}}else if(e.nodeType===t.nodeType)return e.nodeValue!==t.nodeValue&&(e.nodeValue=t.nodeValue),e;return e.replaceWith(t),t}function O(n,e,t,i){if(t instanceof Element&&n.idSets.has(t)){let o=(e.ownerDocument??document).createElementNS(t.namespaceURI,t.localName);return e.insertBefore(o,i),b(n,o,t)}return e.insertBefore(t,i),t}function P(n,e,t){let i=n;if(i.moveBefore&&e.isConnected)try{i.moveBefore(e,t);return}catch{}n.insertBefore(e,t)}function U(n,e){let t=new Set;if(e===n.active&&t.add("value"),!n.options.preserveFormState)return t;if(e instanceof HTMLInputElement)t.add("value"),t.add("checked");else if(e instanceof HTMLTextAreaElement)t.add("value");else if(e instanceof HTMLSelectElement)n.preservedSelects.add(e),t.add("value");else if(e instanceof HTMLOptionElement){let i=e.closest("select");i&&n.preservedSelects.has(i)&&t.add("selected")}return t}function B(n,e,t){for(let i of Array.from(e.attributes))t.has(i.name)||(i.namespaceURI===null?n.getAttribute(i.name)!==i.value&&n.setAttribute(i.name,i.value):n.getAttributeNS(i.namespaceURI,i.localName)!==i.value&&n.setAttributeNS(i.namespaceURI,i.name,i.value));for(let i of Array.from(n.attributes))t.has(i.name)||(i.namespaceURI===null?e.hasAttribute(i.name)||n.removeAttribute(i.name):e.hasAttributeNS(i.namespaceURI,i.localName)||n.removeAttributeNS(i.namespaceURI,i.localName))}function $(n,e,t){n instanceof HTMLInputElement&&e instanceof HTMLInputElement?(!t.has("checked")&&n.checked!==e.checked&&(n.checked=e.checked),!t.has("value")&&n.value!==e.value&&(n.value=e.value)):n instanceof HTMLTextAreaElement&&e instanceof HTMLTextAreaElement?!t.has("value")&&n.value!==e.value&&(n.value=e.value):n instanceof HTMLOptionElement&&e instanceof HTMLOptionElement?!t.has("selected")&&n.selected!==e.selected&&(n.selected=e.selected):n instanceof HTMLSelectElement&&e instanceof HTMLSelectElement&&!t.has("value")&&n.value!==e.value&&(n.value=e.value)}function M(n){let e=i=>JSON.stringify(Array.from(i.scripts,r=>r.outerHTML)),t=i=>{let r=i.doctype;return JSON.stringify(r&&[r.name,r.publicId,r.systemId])};return e(document)===e(n)&&t(document)===t(n)&&document.querySelector("base")?.outerHTML===n.querySelector("base")?.outerHTML}function T(n){let e=document.documentElement,t=n.documentElement,{scrollX:i,scrollY:r}=window;for(let o of Array.from(e.attributes))t.hasAttribute(o.name)||e.removeAttribute(o.name);for(let o of Array.from(t.attributes))e.setAttribute(o.name,o.value);x(e,null,null,t.childNodes,{preserveFormState:!0}),window.scrollTo({left:i,top:r,behavior:"instant"})}var m=class{constructor(e,t,i){this.navigating=e;this.beforeUpdate=t;this.reportError=i}navigating;beforeUpdate;reportError;controller=null;cancel(){this.controller?.abort(),this.controller=null}async refresh(){if(this.cancel(),this.navigating())return;let e=new AbortController;this.controller=e;let t=location.href,i=()=>this.controller===e&&!this.navigating()&&location.href===t;try{if(await q(e.signal),!i())return;let r={};window.dispatchEvent(new CustomEvent(S,{detail:r}));let o=await(r.runtime?.request(e.signal)??fetch(t,{cache:"no-store",headers:{Accept:"text/html"},signal:e.signal}));if(!i())return;if(o.redirected){location.assign(o.url);return}if(!o.ok)throw new Error(`Refresh failed: ${o.status} ${o.statusText}`);if(o.headers.get("Content-Type")?.split(";")[0]?.trim()!=="text/html"){location.reload();return}let s=await o.text();if(!i())return;let l=new DOMParser().parseFromString(s,"text/html");if(!M(l)){location.reload();return}this.beforeUpdate();let d=()=>T(l);r.runtime?r.runtime.replace(d):d()}catch(r){i()&&this.reportError(r)}finally{this.controller===e&&(this.controller=null)}}};function q(n){return document.readyState!=="loading"||n.aborted?Promise.resolve():new Promise(e=>{let t=()=>{document.removeEventListener("DOMContentLoaded",t),n.removeEventListener("abort",t),e()};document.addEventListener("DOMContentLoaded",t,{once:!0}),n.addEventListener("abort",t,{once:!0})})}var p=class{constructor(e){this.enabled=e;if(e){document.addEventListener("DOMContentLoaded",()=>this.render(),{once:!0});for(let t of["400","600"]){let i=new FontFace(L,`url(${F}${t}-normal.woff2) format("woff2")`,{weight:t,display:"swap"});document.fonts.add(i),i.load().catch(()=>{})}}}enabled;pill=null;current=null;show(e,t=!1){this.enabled&&(this.current={label:e,isError:t},this.render())}hide(){this.current=null,this.pill?.host.remove()}render(){if(!this.current||!document.body)return;this.pill??=this.createPill();let{host:e,label:t,spinner:i}=this.pill;t.textContent=this.current.label,t.className=this.current.isError?"error":"busy",i.style.display=this.current.isError?"none":"",e.isConnected||document.body.append(e)}createPill(){let e=document.createElement("topcoat-dev-status");e.style.cssText=W;let t=e.attachShadow({mode:"open"});t.innerHTML=z;let i=t.querySelector("b"),r=t.querySelector(".spinner");return t.querySelector("button").addEventListener("click",()=>this.hide()),{host:e,label:i,spinner:r}}},E="#a1a1aa",N="#fca5a5",L="Topcoat Dev",F="https://cdn.jsdelivr.net/fontsource/fonts/lexend-deca@latest/latin-",k=n=>`<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">${n}</svg>`,_=k('<path d="M18 6 6 18"/><path d="m6 6 12 12"/>'),V=k('<path d="M21 12a9 9 0 1 1-6.219-8.56"/>'),W=`
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
  font: 12px/1 "${L}", ui-sans-serif, system-ui, sans-serif;
  -webkit-font-smoothing: antialiased;
  user-select: none;
`,z=`
  <style>
    .brand {
      color: ${E};
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
      color: ${E};
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
        linear-gradient(100deg, ${N} 30%, #fecaca 50%, ${N} 70%);
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
  <span class="spinner">${V}</span>
  <button class="dismiss" aria-label="Dismiss">${_}</button>
`;function j(n){let e=new p(n.dataset.statusIndicator!=="false"),t=new h(n.src,{event:o=>r[o](),reconnected:()=>{i.refresh()},navigating:()=>i.cancel()}),i=new m(()=>t.isNavigating,()=>e.hide(),o=>{console.error("[topcoat dev]",o),e.show("refresh failed",!0)}),r={reload:()=>{i.refresh()},rebuilding:()=>{i.cancel(),e.show("rebuilding")},"build-failed":()=>e.show("build failed",!0),"app-exited":()=>e.show("app exited",!0),"up-to-date":()=>e.hide()};t.start()}var w=document.currentScript;w instanceof HTMLScriptElement&&j(w);})();
