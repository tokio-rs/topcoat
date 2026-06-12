import { scan } from "./scan";
import { Scope } from "./scope";

scan(document.documentElement, null, null, new Scope(null));
