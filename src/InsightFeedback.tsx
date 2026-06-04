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
  VisualTrajectoryPoint,
  VisualWindowSummary,
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
  const latestWindowSummary = analysisStatus?.latestWindowSummary;
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
              ? "每分钟保留高分辨率原图，5 分钟窗口选 1/3/5 分钟三张图分析，5 小时汇总轨迹"
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
          title="5 分钟窗口摘要"
          status={visualStatus}
          cadence="1/3/5 min samples"
        >
          <WindowSummaryContent
            summary={latestWindowSummary}
            fallbackObservation={latestObservation}
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

function WindowSummaryContent({
  summary,
  fallbackObservation,
  canShowText,
}: {
  summary?: VisualWindowSummary;
  fallbackObservation?: VisualObservation;
  canShowText: boolean;
}) {
  if (!summary) {
    if (fallbackObservation) {
      return (
        <LegacyObservationContent
          observation={fallbackObservation}
          canShowText={canShowText}
        />
      );
    }
    return <p className="emptyState">还没有可展示的 5 分钟窗口分析结果。</p>;
  }

  return (
    <div className="aiInsightBody">
      <div className="aiInsightFacts">
        <span>{formatRange(summary.windowStart, summary.windowEnd)}</span>
        <span>{categoryLabel(summary.primaryActivity)}</span>
        <span>{Math.round(summary.confidence * 100)}% confidence</span>
      </div>
      {canShowText ? (
        <>
          <p>{summary.summaryText}</p>
          <div className="categoryMix" aria-label="Window insight labels">
            <span>{switchingLabel(summary.switchingLevel)}</span>
            <span>{loafingLabel(summary.loafingLevel)}</span>
            {summary.projectHints.slice(0, 2).map((hint) => (
              <span key={hint}>{hint}</span>
            ))}
          </div>
          {summary.taskIntent ? (
            <p className="aiInsightHints">任务意图：{summary.taskIntent}</p>
          ) : null}
          <TrajectoryList trajectory={summary.trajectory} />
          <p className="aiInsightHints">
            切换：{summary.switchingEvidence} 摸鱼：{summary.loafingEvidence}
          </p>
        </>
      ) : (
        <p className="redactedText insightRedacted">
          5 分钟窗口摘要已生成，内容在 Redacted 模式隐藏。
        </p>
      )}
      {summary.error ? (
        <p className="aiInsightError">
          <AlertCircle aria-hidden="true" size={15} />
          <span>{summary.error}</span>
        </p>
      ) : null}
    </div>
  );
}

function LegacyObservationContent({
  observation,
  canShowText,
}: {
  observation: VisualObservation;
  canShowText: boolean;
}) {
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
          旧版单张截图摘要已生成，内容在 Redacted 模式隐藏。
        </p>
      )}
    </div>
  );
}

function TrajectoryList({ trajectory }: { trajectory: VisualTrajectoryPoint[] }) {
  if (trajectory.length === 0) {
    return null;
  }

  return (
    <ol className="windowTrajectory" aria-label="1 3 5 minute trajectory">
      {trajectory.map((point) => (
        <li key={`${point.minuteMark}-${point.screenshotId}`}>
          <strong>第 {point.minuteMark} 分钟</strong>
          <span>{point.observation}</span>
        </li>
      ))}
    </ol>
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
        <span>{report.evidenceCount} windows</span>
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

function switchingLabel(level: string): string {
  switch (level) {
    case "low":
      return "低切换";
    case "medium":
      return "中切换";
    case "high":
      return "高切换";
    default:
      return "切换未知";
  }
}

function loafingLabel(level: string): string {
  switch (level) {
    case "none":
      return "无摸鱼";
    case "possible":
      return "可能摸鱼";
    case "clear":
      return "明显摸鱼";
    default:
      return "摸鱼未知";
  }
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
