import { Activity, AlertTriangle, Clock, Database, Monitor, Server } from "lucide-react";
import { useEffect, useState } from "react";
import { fetchCollectorHealth } from "./lib/health";
import type { CollectorHealth, SubsystemHealth } from "./types";

type MonitorStatus = "offline" | "connecting" | "connected";

function formatUptime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Invalid";
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  }
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${bytes} B`;
}

function SubsystemRow({ name, health, icon }: {
  name: string;
  health: SubsystemHealth;
  icon: React.ReactNode;
}) {
  return (
    <div className="subsystemRow">
      <div className="subsystemIcon">{icon}</div>
      <div className="subsystemLabel">{name}</div>
      <span className={`healthPill ${health.status}`}>
        {health.status === "running" ? "Running" : health.status === "error" ? "Error" : "Not Started"}
      </span>
      {health.lastEventAt && (
        <span className="subsystemTime">Last: {formatTime(health.lastEventAt)}</span>
      )}
      {health.errorCount > 0 && (
        <span className="subsystemErrors">{health.errorCount} errors</span>
      )}
    </div>
  );
}

export function CollectorMonitor() {
  const [health, setHealth] = useState<CollectorHealth | null>(null);
  const [status, setStatus] = useState<MonitorStatus>("connecting");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    async function poll() {
      try {
        const h = await fetchCollectorHealth();
        if (active) {
          setHealth(h);
          setStatus("connected");
          setError(null);
        }
      } catch (err) {
        if (active) {
          setStatus("offline");
          setError(err instanceof Error ? err.message : String(err));
        }
      }
    }

    void poll();
    const interval = setInterval(() => void poll(), 5000);
    return () => {
      active = false;
      clearInterval(interval);
    };
  }, []);

  if (status === "connecting") {
    return (
      <div className="panel">
        <div className="panelHeader">
          <Server aria-hidden="true" size={20} />
          <h2>Collector Monitor</h2>
        </div>
        <p className="monitorConnecting">
          <Activity aria-hidden="true" size={16} />
          Connecting to collector...
        </p>
      </div>
    );
  }

  if (status === "offline") {
    return (
      <div className="panel">
        <div className="panelHeader">
          <Server aria-hidden="true" size={20} />
          <h2>Collector Monitor</h2>
        </div>
        <dl className="statusList">
          <div>
            <dt>Status</dt>
            <dd>
              <span className="healthPill error">Offline</span>
            </dd>
          </div>
        </dl>
        <p className="offlineHint">
          Collector not reachable. Run <code>start.bat</code> or{" "}
          <code>npm run collector</code> to start.
        </p>
        {error && (
          <p className="errors" role="status">
            {error}
          </p>
        )}
      </div>
    );
  }

  // Connected
  const anyError = health?.windowCollector.lastError
    || health?.inputCollector.lastError
    || health?.screenshotCollector.lastError;

  return (
    <div className="panel collectorMonitor">
      <div className="panelHeader">
        <Server aria-hidden="true" size={20} />
        <h2>Collector Monitor</h2>
      </div>

      <div className="healthGrid">
        <div className="healthOverview">
          <span className={`healthPill ${health!.status}`}>
            {health!.status === "ok" ? "Healthy" : health!.status === "degraded" ? "Degraded" : "Error"}
          </span>
          <span className="healthUptime">
            <Clock aria-hidden="true" size={14} />
            Uptime {formatUptime(health!.uptimeSeconds)}
          </span>
        </div>
        <div className="healthMeta">
          <span>v{health!.version}</span>
          <span>Started {formatTime(health!.startedAt)}</span>
        </div>
      </div>

      <div className="monitorSection">
        <div className="monitorSectionHeader">
          <Monitor aria-hidden="true" size={16} />
          <h3>Subsystems</h3>
        </div>
        <div className="subsystemList">
          <SubsystemRow
            name="Window Collector"
            health={health!.windowCollector}
            icon={<Activity aria-hidden="true" size={14} />}
          />
          <SubsystemRow
            name="Input Collector"
            health={health!.inputCollector}
            icon={<Activity aria-hidden="true" size={14} />}
          />
          <SubsystemRow
            name="Screenshot Collector"
            health={health!.screenshotCollector}
            icon={<Activity aria-hidden="true" size={14} />}
          />
        </div>
      </div>

      <div className="monitorSection">
        <div className="monitorSectionHeader">
          <Database aria-hidden="true" size={16} />
          <h3>Database</h3>
        </div>
        <div className="dbStatList">
          <div className="dbStatRow">
            <span>Window Events</span>
            <strong>{health!.dbStats.windowEvents.toLocaleString()}</strong>
          </div>
          <div className="dbStatRow">
            <span>Input Events</span>
            <strong>{health!.dbStats.inputEvents.toLocaleString()}</strong>
          </div>
          <div className="dbStatRow">
            <span>Text Segments</span>
            <strong>{health!.dbStats.textSegments.toLocaleString()}</strong>
          </div>
          <div className="dbStatRow">
            <span>Screenshots</span>
            <strong>{health!.dbStats.screenshots.toLocaleString()}</strong>
          </div>
          <div className="dbStatRow">
            <span>High-res screenshots</span>
            <strong>{health!.dbStats.highResScreenshots.toLocaleString()}</strong>
          </div>
          <div className="dbStatRow">
            <span>Blocker Hits</span>
            <strong>{health!.dbStats.blockerHits.toLocaleString()}</strong>
          </div>
        </div>
      </div>

      <div className="monitorSection">
        <div className="monitorSectionHeader">
          <Database aria-hidden="true" size={16} />
          <h3>Image Retention</h3>
          <span className="retentionBadge">
            {health!.dbStats.imageRetention.retentionDays} days local
          </span>
        </div>
        <div className="dbStatList">
          <div className="dbStatRow">
            <span>Active files</span>
            <strong>{health!.dbStats.imageRetention.activeFiles.toLocaleString()}</strong>
          </div>
          <div className="dbStatRow">
            <span>Active size</span>
            <strong>{formatBytes(health!.dbStats.imageRetention.activeBytes)}</strong>
          </div>
          <div className="dbStatRow">
            <span>Expired files</span>
            <strong>{health!.dbStats.imageRetention.expiredFiles.toLocaleString()}</strong>
          </div>
        </div>
        {health!.dbStats.imageRetention.pendingGoogleDriveUpload && (
          <div className="retentionNotice">
            <AlertTriangle aria-hidden="true" size={16} />
            <p>
              {health!.dbStats.imageRetention.googleDriveMessage ??
                "Local screenshots are temporary. Upload older evidence to Google Drive before cleanup."}
            </p>
          </div>
        )}
      </div>

      {anyError && (
        <div className="errorBanner">
          <AlertTriangle aria-hidden="true" size={16} />
          <div>
            {health?.windowCollector.lastError && (
              <p>Window: {health.windowCollector.lastError}</p>
            )}
            {health?.inputCollector.lastError && (
              <p>Input: {health.inputCollector.lastError}</p>
            )}
            {health?.screenshotCollector.lastError && (
              <p>Screenshot: {health.screenshotCollector.lastError}</p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
