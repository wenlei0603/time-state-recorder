import {
  AlertCircle,
  Camera,
  Clock3,
  FileText,
  Sparkles,
} from "lucide-react";
import type { ReactNode } from "react";
import type { PrivacyMode, UiSourceMode } from "./lib/uiModel";
import type {
  ActivityCategory,
  AnalysisStatus,
  AnalysisWorkerStatus,
  InsightReport,
  VisualObservation,
} from "./types";

type InsightFeedbackProps = {
  analysisStatus?: AnalysisStatus;
  reports: InsightReport[];
  privacyMode: PrivacyMode;
  sourceMode: UiSourceMode;
  loading: boolean;
  error?: string | null;
};

export function InsightFeedback({
  analysisStatus,
  reports,
  privacyMode,
  sourceMode,
  loading,
  error,
}: InsightFeedbackProps) {
  const latestObservation = analysisStatus?.latestObservation;
  const latestReport = analysisStatus?.latestReport ?? reports[0];
  const visualStatus = analysisStatus?.visual;
  const reportStatus = analysisStatus?.report;
  const canShowText = privacyMode === "raw";

  return (
    <section className="aiInsightPanel" aria-label="AI insight feedback">
      <div className="aiInsightHeader">
        <div>
          <p className="eyebrow">AI insight</p>
          <h2>AI 工作洞察</h2>
          <p>
            {sourceMode === "live"
              ? "高分辨率截图每 5 分钟分析一次，5 小时生成一次轨迹报告"
              : "等待 Live collector 后开始反馈"}
          </p>
        </div>
        <span className={`statusPill ${statusClass(visualStatus?.status)}`}>
          {loading ? "Refreshing" : workerLabel(visualStatus)}
        </span>
      </div>

      {error ? (
        <p className="errors" role="status">
          {error}
        </p>
      ) : null}

      <div className="aiInsightGrid">
        <InsightBlock
          icon={<Camera aria-hidden="true" size={18} />}
          title="截图摘要"
          status={visualStatus}
          cadence="5 min"
        >
          <ObservationContent
            observation={latestObservation}
            canShowText={canShowText}
          />
        </InsightBlock>

        <InsightBlock
          icon={<FileText aria-hidden="true" size={18} />}
          title="5h report"
          status={reportStatus}
          cadence="5 h"
        >
          <ReportContent report={latestReport} canShowText={canShowText} />
        </InsightBlock>

        <div className="aiInsightTiming">
          <Clock3 aria-hidden="true" size={18} />
          <div>
            <strong>下一次运行</strong>
            <span>截图分析：{formatTime(visualStatus?.nextRunAt)}</span>
            <span>轨迹报告：{formatTime(reportStatus?.nextRunAt)}</span>
          </div>
        </div>
      </div>
    </section>
  );
}

function InsightBlock({
  icon,
  title,
  status,
  cadence,
  children,
}: {
  icon: ReactNode;
  title: string;
  status?: AnalysisWorkerStatus;
  cadence: string;
  children: ReactNode;
}) {
  return (
    <article className="aiInsightBlock">
      <div className="aiInsightBlockHeader">
        <span className="metricIcon">{icon}</span>
        <div>
          <h3>{title}</h3>
          <span>{cadence} cadence</span>
        </div>
        <span className={`workerBadge ${statusClass(status?.status)}`}>
          {workerLabel(status)}
        </span>
      </div>
      {status?.lastError ? (
        <p className="aiInsightError">
          <AlertCircle aria-hidden="true" size={15} />
          <span>{status.lastError}</span>
        </p>
      ) : null}
      {children}
    </article>
  );
}

function ObservationContent({
  observation,
  canShowText,
}: {
  observation?: VisualObservation;
  canShowText: boolean;
}) {
  if (!observation) {
    return <p className="emptyState">还没有可展示的截图分析结果。</p>;
  }

  return (
    <div className="aiInsightBody">
      <div className="aiInsightFacts">
        <span>{formatDateTime(observation.capturedAt)}</span>
        <span>{categoryLabel(observation.activityCategory)}</span>
        <span>{Math.round(observation.confidence * 100)}% confidence</span>
      </div>
      {canShowText ? (
        <p>{observation.summaryText}</p>
      ) : (
        <p className="redactedText insightRedacted">
          模型摘要已生成，内容在 Redacted 模式隐藏。
        </p>
      )}
      {observation.error ? (
        <p className="aiInsightError">
          <AlertCircle aria-hidden="true" size={15} />
          <span>{observation.error}</span>
        </p>
      ) : null}
    </div>
  );
}

function ReportContent({
  report,
  canShowText,
}: {
  report?: InsightReport;
  canShowText: boolean;
}) {
  if (!report) {
    return <p className="emptyState">5 小时轨迹报告尚未生成。</p>;
  }

  return (
    <div className="aiInsightBody">
      <div className="aiInsightFacts">
        <span>{formatRange(report.periodStart, report.periodEnd)}</span>
        <span>{report.evidenceCount} screenshots</span>
        <span>{report.modelProvider}</span>
      </div>
      {report.categoryMix.length > 0 ? (
        <div className="categoryMix" aria-label="5h report category mix">
          {report.categoryMix.slice(0, 4).map((item) => (
            <span key={item.activityCategory}>
              {categoryLabel(item.activityCategory)} {item.count}
            </span>
          ))}
        </div>
      ) : null}
      {canShowText ? (
        <>
          <p>{report.summaryText}</p>
          {report.projectHints.length > 0 ? (
            <p className="aiInsightHints">
              {report.projectHints.slice(0, 5).join(" / ")}
            </p>
          ) : null}
        </>
      ) : (
        <p className="redactedText insightRedacted">
          5h report 已生成，正文在 Redacted 模式隐藏。
        </p>
      )}
      {report.error ? (
        <p className="aiInsightError">
          <AlertCircle aria-hidden="true" size={15} />
          <span>{report.error}</span>
        </p>
      ) : null}
    </div>
  );
}

function statusClass(status?: string): string {
  if (status === "running") {
    return "loading";
  }
  if (status === "error") {
    return "offline";
  }
  return "connected";
}

function workerLabel(status?: AnalysisWorkerStatus): string {
  if (!status) {
    return "Pending";
  }
  if (status.status === "running") {
    return "Running";
  }
  if (status.status === "error") {
    return "Error";
  }
  return "Idle";
}

function formatTime(value?: string): string {
  if (!value) {
    return "pending";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "invalid";
  }
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function formatDateTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString([], {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatRange(start: string, end: string): string {
  return `${formatTime(start)} - ${formatTime(end)}`;
}

function categoryLabel(category: ActivityCategory): string {
  switch (category) {
    case "project_work":
      return "Project";
    case "research":
      return "Research";
    case "writing":
      return "Writing";
    case "coding":
      return "Coding";
    case "communication":
      return "Comms";
    case "meeting":
      return "Meeting";
    case "admin":
      return "Admin";
    case "learning":
      return "Learning";
    case "planning":
      return "Planning";
    case "loafing":
      return "Loafing";
    case "personal":
      return "Personal";
    case "idle":
      return "Idle";
    case "unknown":
      return "Unknown";
  }
}
