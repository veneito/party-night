// Gives the TV page a tiny, safe bridge to the app for update checks.
const { contextBridge, ipcRenderer } = require('electron');
contextBridge.exposeInMainWorld('partyApp', {
  version: () => ipcRenderer.invoke('version'),
  check: () => ipcRenderer.send('update:check'),
  download: () => ipcRenderer.send('update:download'),
  onUpdate: fn => ipcRenderer.on('update', (_, u) => fn(u)),
});
