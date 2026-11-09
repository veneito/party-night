// Party Night desktop app: runs the game server and shows the TV screen in its own window.
// Phones on the same Wi-Fi join at the address shown on screen.
const { app, BrowserWindow, Menu, shell, ipcMain } = require('electron');
const path = require('path');
const { start } = require('./server.js');

// sound effects and read-aloud jokes should play without a click first
app.commandLine.appendSwitch('autoplay-policy', 'no-user-gesture-required');

// a second launch just focuses the first window instead of starting another server
if (!app.requestSingleInstanceLock()) app.quit();

let win;
app.whenReady().then(async () => {
  let port;
  try { port = await start(); } catch (e) {
    const { dialog } = require('electron');
    dialog.showErrorBox('Party Night', `Couldn't start the game server:\n${e.message}`);
    return app.quit();
  }
  Menu.setApplicationMenu(null);
  win = new BrowserWindow({
    width: 1600, height: 900, minWidth: 960, minHeight: 540,
    title: 'Party Night', backgroundColor: '#0f0b1e', autoHideMenuBar: true, show: false,
    webPreferences: { preload: path.join(__dirname, 'preload.js') },
  });
  win.loadURL(`http://localhost:${port}/tv`);
  win.once('ready-to-show', () => { win.maximize(); win.show(); });
  setupUpdates();
  // F11 or Escape toggles fullscreen, the way a TV app should
  win.webContents.on('before-input-event', (e, input) => {
    if (input.type !== 'keyDown') return;
    if (input.key === 'F11') win.setFullScreen(!win.isFullScreen());
    else if (input.key === 'Escape' && win.isFullScreen()) win.setFullScreen(false);
    else if (input.key === 'F5') win.reload();
  });
  // any outside link opens in the normal browser, not inside the game window
  win.webContents.setWindowOpenHandler(({ url }) => { shell.openExternal(url); return { action: 'deny' }; });
});
app.on('second-instance', () => { if (win) { win.isMinimized() && win.restore(); win.focus(); } });
app.on('window-all-closed', () => app.quit());

// Updates come from GitHub releases. The app asks first (popup on the TV screen); on "Update" it downloads,
// installs silently and reopens itself.
ipcMain.handle('version', () => app.getVersion());
function setupUpdates() {
  const tell = u => win?.webContents.send('update', u);
  if (!app.isPackaged) {
    ipcMain.on('update:check', () => tell({ state: 'error', message: 'updates only work in the installed app' }));
    return;
  }
  const { autoUpdater } = require('electron-updater');
  autoUpdater.autoDownload = false;
  autoUpdater.on('update-available', i => tell({ state: 'available', version: i.version }));
  autoUpdater.on('update-not-available', () => tell({ state: 'none' }));
  autoUpdater.on('download-progress', p => tell({ state: 'progress', percent: Math.round(p.percent) }));
  autoUpdater.on('update-downloaded', () => { tell({ state: 'installing' }); setTimeout(() => autoUpdater.quitAndInstall(true, true), 1500); });
  autoUpdater.on('error', e => tell({ state: 'error', message: e.message.split('
')[0] }));
  ipcMain.on('update:check', () => autoUpdater.checkForUpdates().catch(() => {}));
  ipcMain.on('update:download', () => autoUpdater.downloadUpdate().catch(() => {}));
  // check once shortly after opening, so the popup shows without anyone asking
  setTimeout(() => autoUpdater.checkForUpdates().catch(() => {}), 5000);
}
