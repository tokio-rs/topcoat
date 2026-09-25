import { Runtime } from "./runtime";

const runtime = new Runtime();
runtime.start(document);
runtime.page.listenForDevRefresh();
