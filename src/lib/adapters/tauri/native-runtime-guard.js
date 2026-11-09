/** @typedef {'dashboard' | 'market data' | 'screener' | 'audit history' | 'strict analysis' | 'paper order' | 'backtest' | 'agent lifecycle' | 'risk controls' | 'account maintenance' | 'risk settings' | 'research workspace' | 'MT5 bridge' | 'MT5 bridge setup' | 'MT5 market-data series' | 'MT5 instrument specification' | 'MT5 session calendar' | 'technical indicators' | 'native indicator signals' | 'market structure analysis' | 'MT5 risk sizing' | 'market-data provider registry' | 'unified market-data query' | 'meme coin hunter'} NativeCapability */

export class NativeRuntimeRequiredError extends Error {
  /** @param {NativeCapability} capability */
  constructor(capability) {
    super(`${capability} requires the installed Tauri desktop runtime. Browser execution is disabled; no synthetic market data or audit records are available.`);
    this.name = 'NativeRuntimeRequiredError';
    /** @type {'NATIVE_RUNTIME_REQUIRED'} */
    this.code = 'NATIVE_RUNTIME_REQUIRED';
  }
}

/**
 * Deny all application capabilities when the Rust/Tauri boundary is absent.
 * This is shared by every bridge operation so adding a command cannot silently
 * reintroduce a browser-only data path.
 * @param {boolean} available
 * @param {NativeCapability} capability
 * @returns {void}
 */
export function assertNativeRuntime(available, capability) {
  if (available === true) return;
  throw new NativeRuntimeRequiredError(capability);
}
