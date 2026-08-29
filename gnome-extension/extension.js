import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

import {stateLabel, validatePayload} from './protocol.js';

const PAYLOAD_NAME = 'gnome-presentation-v1.json';

export default class CookbenchPresentationExtension extends Extension {
  enable() {
    this._box = new St.BoxLayout({
      style_class: 'cookbench-stove-list',
      reactive: false,
    });
    Main.panel._centerBox.insert_child_at_index(this._box, 0);

    this._payloadFile = Gio.File.new_for_path(GLib.build_filenamev([
      GLib.get_user_runtime_dir(),
      'cookbench',
      PAYLOAD_NAME,
    ]));
    try {
      this._monitor = this._payloadFile.monitor_file(Gio.FileMonitorFlags.NONE, null);
      this._changedId = this._monitor.connect('changed', () => this._loadPayload());
    } catch (_) {
      // A missing Cookbench runtime directory is normal before the app starts.
      this._monitor = null;
      this._changedId = null;
    }
    this._retryId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 2, () => {
      this._loadPayload();
      return GLib.SOURCE_CONTINUE;
    });
    this._loadPayload();
  }

  disable() {
    if (this._retryId)
      GLib.Source.remove(this._retryId);
    this._retryId = null;
    if (this._monitor) {
      if (this._changedId)
        this._monitor.disconnect(this._changedId);
      this._monitor.cancel();
    }
    this._changedId = null;
    this._monitor = null;
    this._payloadFile = null;
    this._box?.destroy();
    this._box = null;
  }

  _loadPayload() {
    if (!this._payloadFile || !this._box)
      return;
    try {
      const [ok, bytes] = this._payloadFile.load_contents(null);
      if (!ok)
        return this._render(null);
      this._render(validatePayload(JSON.parse(new TextDecoder().decode(bytes))));
    } catch (_) {
      // Cookbench may be absent, updating atomically, or have been removed.
      // Presentation vanishes cleanly; no source state is read as a fallback.
      this._render(null);
    }
  }

  _render(payload) {
    this._box.destroy_all_children();
    if (!payload)
      return;

    for (const stove of payload.stoves) {
      const progress = stove.progress ? ` ${stove.progress.completed}/${stove.progress.total}` : '';
      const label = new St.Label({
        style_class: `cookbench-stove cookbench-state-${stove.state}`,
        text: `${stove.harness} ${stove.project} ${stateLabel(stove.state)}${progress}`,
      });
      this._box.add_child(label);
    }
  }
}
