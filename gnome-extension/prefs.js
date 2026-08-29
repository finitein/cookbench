import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

// The extension has no independent settings. Cookbench remains authoritative
// for its own presentation state and all user configuration.
export default class CookbenchPresentationPreferences extends ExtensionPreferences {}
