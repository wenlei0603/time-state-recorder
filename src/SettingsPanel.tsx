import { AlertTriangle, Database, KeyRound, RotateCw, Save, Server } from "lucide-react";
import type { FormEvent } from "react";
import { useEffect, useState } from "react";
import { fetchAppConfig, saveAppConfig } from "./lib/config";
import type { AppConfig, AppConfigPatch } from "./types";

type SettingsStatus = "loading" | "ready" | "saving" | "error";

export function SettingsPanel() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [status, setStatus] = useState<SettingsStatus>("loading");
  const [message, setMessage] = useState<string | null>(null);
  const [restartRequired, setRestartRequired] = useState(false);
  const [restartReasons, setRestartReasons] = useState<string[]>([]);

  useEffect(() => {
    let active = true;
    async function load() {
      setStatus("loading");
      try {
        const response = await fetchAppConfig();
        if (!active) return;
        setConfig(response.config);
        setRestartRequired(response.restartRequired);
        setRestartReasons(response.restartReasons);
        setMessage(null);
        setStatus("ready");
      } catch (error) {
        if (!active) return;
        setMessage(error instanceof Error ? error.message : String(error));
        setStatus("error");
      }
    }
    void load();
    return () => {
      active = false;
    };
  }, []);

  if (status === "loading" && !config) {
    return (
      <section className="workspace settingsWorkspace">
        <div className="panel settingsPanel">
          <div className="panelHeader">
            <Server aria-hidden="true" size={20} />
            <h2>Settings</h2>
          </div>
          <p className="monitorConnecting">Loading settings...</p>
        </div>
      </section>
    );
  }

  if (!config) {
    return (
      <section className="workspace settingsWorkspace">
        <div className="panel settingsPanel">
          <div className="panelHeader">
            <AlertTriangle aria-hidden="true" size={20} />
            <h2>Settings</h2>
          </div>
          <p className="errors" role="status">{message ?? "Settings unavailable"}</p>
        </div>
      </section>
    );
  }

  function updateConfig(next: Partial<AppConfig>) {
    setConfig((current) => (current ? { ...current, ...next } : current));
  }

  function updateStorage(key: keyof AppConfig["storage"], value: string) {
    updateConfig({ storage: { ...config!.storage, [key]: value } });
  }

  function updateRuntime(key: keyof AppConfig["runtime"], value: string) {
    const parsed = key === "pollMs" ? Number(value) : value;
    updateConfig({ runtime: { ...config!.runtime, [key]: parsed } });
  }

  function updateCapture(key: keyof AppConfig["capture"], value: string) {
    updateConfig({ capture: { ...config!.capture, [key]: Number(value) } });
  }

  function updateVisual(key: keyof AppConfig["visual"], value: string) {
    const parsed = key === "maxCompletionTokens" ? Number(value) : value;
    updateConfig({ visual: { ...config!.visual, [key]: parsed } });
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const patch: AppConfigPatch = {
      storage: config!.storage,
      runtime: config!.runtime,
      capture: config!.capture,
      visual: {
        provider: config!.visual.provider,
        baseUrl: config!.visual.baseUrl,
        model: config!.visual.model,
        imageDetail: config!.visual.imageDetail,
        maxCompletionTokens: config!.visual.maxCompletionTokens,
      },
    };
    if (apiKey.trim()) {
      patch.visual = { ...patch.visual, apiKey: apiKey.trim() };
    }

    setStatus("saving");
    setMessage(null);
    try {
      const response = await saveAppConfig(patch);
      setConfig(response.config);
      setApiKey("");
      setRestartRequired(response.restartRequired);
      setRestartReasons(response.restartReasons);
      setMessage("Settings saved");
      setStatus("ready");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
      setStatus("error");
    }
  }

  return (
    <section className="workspace settingsWorkspace">
      <form className="settingsPanel" onSubmit={handleSubmit}>
        <div className="settingsHeader">
          <div>
            <p className="eyebrow">Collector configuration</p>
            <h2>Settings</h2>
          </div>
          <div className="inlineActions">
            <button
              type="button"
              className="iconButton"
              onClick={() => window.location.reload()}
              title="Reload settings"
            >
              <RotateCw aria-hidden="true" size={16} />
              <span>Reload</span>
            </button>
            <button type="submit" className="iconButton" disabled={status === "saving"}>
              <Save aria-hidden="true" size={16} />
              <span>Save settings</span>
            </button>
          </div>
        </div>

        {(message || restartRequired) && (
          <div className={`settingsNotice ${restartRequired ? "restart" : ""}`} role="status">
            {restartRequired ? (
              <>
                <AlertTriangle aria-hidden="true" size={16} />
                <span>
                  Restart required
                  {restartReasons.length > 0 ? `: ${restartReasons.join(", ")}` : ""}
                </span>
              </>
            ) : (
              <span>{message}</span>
            )}
          </div>
        )}

        <div className="settingsGrid">
          <section className="settingsSection">
            <div className="monitorSectionHeader">
              <KeyRound aria-hidden="true" size={16} />
              <h3>API Provider</h3>
            </div>
            <div className="settingsFields">
              <label>
                <span>Provider</span>
                <select
                  value={config.visual.provider}
                  onChange={(event) => updateVisual("provider", event.currentTarget.value)}
                >
                  <option value="local">Local metadata</option>
                  <option value="minimax">MiniMax</option>
                </select>
              </label>
              <label>
                <span>Base URL</span>
                <input
                  value={config.visual.baseUrl ?? ""}
                  onChange={(event) => updateVisual("baseUrl", event.currentTarget.value)}
                  placeholder="https://api.minimax.io/v1"
                />
              </label>
              <label>
                <span>Model</span>
                <input
                  value={config.visual.model}
                  onChange={(event) => updateVisual("model", event.currentTarget.value)}
                />
              </label>
              <label>
                <span>API key</span>
                <input
                  aria-label="API key"
                  type="password"
                  value={apiKey}
                  onChange={(event) => setApiKey(event.currentTarget.value)}
                  placeholder="Leave blank to keep current key"
                />
              </label>
              {config.visual.apiKeyMasked && (
                <p className="settingsHint">
                  Current key: <code>{config.visual.apiKeyMasked}</code>
                </p>
              )}
              <label>
                <span>Image detail</span>
                <select
                  value={config.visual.imageDetail}
                  onChange={(event) => updateVisual("imageDetail", event.currentTarget.value)}
                >
                  <option value="low">Low</option>
                  <option value="default">Default</option>
                  <option value="high">High</option>
                </select>
              </label>
              <label>
                <span>Max completion tokens</span>
                <input
                  type="number"
                  min={1}
                  value={config.visual.maxCompletionTokens}
                  onChange={(event) =>
                    updateVisual("maxCompletionTokens", event.currentTarget.value)
                  }
                />
              </label>
            </div>
          </section>

          <section className="settingsSection">
            <div className="monitorSectionHeader">
              <Database aria-hidden="true" size={16} />
              <h3>Storage</h3>
            </div>
            <div className="settingsFields">
              <label>
                <span>Database path</span>
                <input
                  aria-label="Database path"
                  value={config.storage.databasePath}
                  onChange={(event) => updateStorage("databasePath", event.currentTarget.value)}
                />
              </label>
              <label>
                <span>Screenshot directory</span>
                <input
                  value={config.storage.screenshotDir}
                  onChange={(event) => updateStorage("screenshotDir", event.currentTarget.value)}
                />
              </label>
              <label>
                <span>High-res screenshot directory</span>
                <input
                  value={config.storage.highResScreenshotDir}
                  onChange={(event) =>
                    updateStorage("highResScreenshotDir", event.currentTarget.value)
                  }
                />
              </label>
            </div>
          </section>

          <section className="settingsSection">
            <div className="monitorSectionHeader">
              <Server aria-hidden="true" size={16} />
              <h3>Runtime</h3>
            </div>
            <div className="settingsFields twoColumnFields">
              <label>
                <span>API address</span>
                <input
                  value={config.runtime.apiAddr}
                  onChange={(event) => updateRuntime("apiAddr", event.currentTarget.value)}
                />
              </label>
              <label>
                <span>Poll ms</span>
                <input
                  type="number"
                  min={100}
                  value={config.runtime.pollMs}
                  onChange={(event) => updateRuntime("pollMs", event.currentTarget.value)}
                />
              </label>
              <label>
                <span>Screenshot interval</span>
                <input
                  type="number"
                  min={1}
                  value={config.capture.screenshotIntervalSecs}
                  onChange={(event) =>
                    updateCapture("screenshotIntervalSecs", event.currentTarget.value)
                  }
                />
              </label>
              <label>
                <span>High-res interval</span>
                <input
                  type="number"
                  min={1}
                  value={config.capture.highResScreenshotIntervalSecs}
                  onChange={(event) =>
                    updateCapture("highResScreenshotIntervalSecs", event.currentTarget.value)
                  }
                />
              </label>
              <label>
                <span>Idle threshold</span>
                <input
                  type="number"
                  min={1}
                  value={config.capture.idleThresholdSecs}
                  onChange={(event) =>
                    updateCapture("idleThresholdSecs", event.currentTarget.value)
                  }
                />
              </label>
            </div>
          </section>
        </div>
      </form>
    </section>
  );
}
