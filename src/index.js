import { Plugin } from "obsidian";
import rustPlugin from "../pkg/marksage_bg.wasm";
import * as plugin from "../pkg/marksage.js";

export default class RustPlugin extends Plugin {
  async onload() {
    const buffer = Uint8Array.from(atob(rustPlugin), (c) => c.charCodeAt(0));
    await plugin.default(Promise.resolve(buffer));
    plugin.onload(this);
  }
}
