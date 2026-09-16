"use strict";(()=>{(()=>{let i=document.currentScript;if(!(i instanceof HTMLScriptElement))return;let l=(()=>{let e=new URL(i.src);return e.protocol=e.protocol==="https:"?"wss:":"ws:",e.pathname="/ws",e.search="",e.hash="",e.href})(),d=i.dataset.statusIndicator!=="false",p="#a1a1aa",u="#fca5a5",f="Topcoat Dev",b="https://cdn.jsdelivr.net/fontsource/fonts/lexend-deca@latest/latin-";if(d)for(let e of["400","600"]){let t=new FontFace(f,`url(${b}${e}-normal.woff2) format("woff2")`,{weight:e,display:"swap"});document.fonts.add(t),t.load().catch(()=>{})}let h=e=>`<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">${e}</svg>`,x=h('<path d="M18 6 6 18"/><path d="m6 6 12 12"/>'),y=h('<path d="M21 12a9 9 0 1 1-6.219-8.56"/>'),v=`
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
    font: 12px/1 "${f}", ui-sans-serif, system-ui, sans-serif;
    -webkit-font-smoothing: antialiased;
    user-select: none;
  `,k=`
    <style>
      .brand {
        color: ${p};
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
        color: ${p};
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
          linear-gradient(100deg, ${u} 30%, #fecaca 50%, ${u} 70%);
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
    <span class="spinner">${y}</span>
    <button class="dismiss" aria-label="Dismiss">${x}</button>
  `,r;function E(){let e=document.createElement("topcoat-dev-status");e.style.cssText=v;let t=e.attachShadow({mode:"open"});t.innerHTML=k;let s=t.querySelector("b"),a=t.querySelector(".spinner"),c=t.querySelector("button");return c.onclick=m,{host:e,statusEl:s,spinnerEl:a}}function n(e,t){if(!d)return;if(!document.body){document.addEventListener("DOMContentLoaded",()=>n(e,t),{once:!0});return}r??=E();let{host:s,statusEl:a,spinnerEl:c}=r;a.textContent=e,a.className=t?"error":"busy",c.style.display=t?"none":"",s.isConnected||document.body.append(s)}function m(){r?.host.remove()}let S={reload:()=>window.location.reload(),rebuilding:()=>n("rebuilding",!1),"build-failed":()=>n("build failed",!0),"app-exited":()=>n("app exited",!0),"up-to-date":m};function g(){let e=new WebSocket(l);e.onmessage=t=>S[t.data]?.(),e.onclose=w}let o=!1;window.navigation?.addEventListener("navigate",()=>o=!0),window.navigation?.addEventListener("navigateerror",()=>o=!1),window.addEventListener("pageshow",()=>o=!1);function w(){setTimeout(()=>{if(o)return w();let e=new WebSocket(l);e.onopen=()=>{e.close(),window.location.reload()},e.onerror=()=>setTimeout(g,1e3)},500)}g()})();})();
