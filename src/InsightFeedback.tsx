import { AlertCircle, Camera, Clock3, FileText } from "lucide-react";
import type { ReactNode } from "react";
import {
  buildReportNote,
  buildWindowNote,
  type ReportPhase,
} from "./lib/insightPresentation";
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
    <section className="aiInsightPanel" aria-label="Review Notes">
      <div className="aiInsightHeader">
        <div>
          <p className="eyebrow">Field notes</p>
          <h2>Review Notes</h2>
          <p>
            {sourceMode === "live"
              ? "5 min window · 5 h session · live collector"
              : "5 min window · 5 h session · sample workspace"}
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
          title="Window note"
          status={visualStatus}
          cadence="1 / 3 / 5 min samples"
        >
          <WindowSummaryContent
            summary={latestWindowSummary}
            fallbackObservation={latestObservation}
            canShowText={canShowText}
          />
        </InsightBlock>

        <InsightBlock
          icon={<FileText aria-hidden="true" size={18} />}
          title="Session report"
          status={reportStatus}
          cadence="5 h cadence"
        >
          <ReportContent report={latestReport} canShowText={canShowText} />
        </InsightBlock>
      </div>

      <div className="aiInsightTiming" aria-label="Review note schedule">
        <Clock3 aria-hidden="true" size={17} />
        <span>Window next {formatTime(visualStatus?.nextRunAt)}</span>
        <span>Report next {formatTime(reportStatus?.nextRunAt)}</span>
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
          <span>{cadence}</span>
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
    return <p className="emptyState">No window note yet.</p>;
  }

  const note = buildWindowNote(summary);

  return (
    <div className="aiInsightBody">
      <div className="aiInsightFacts">
        <span>{formatRange(summary.windowStart, summary.windowEnd)}</span>
        <span>{categoryLabel(note.primaryActivity)}</span>
        <span>{Math.round(note.confidence * 100)}% confidence</span>
        <span>{summary.modelProvider}</span>
      </div>
      {canShowText ? (
        <>
          <section
            className="reviewReadableSummary"
            aria-label="Window note summary"
          >
            <p className="reviewThesis">{note.thesis}</p>
            <ReviewFieldGrid
              fields={[
                ["Intent", note.intent],
                ["Continuity", note.continuity],
              ]}
            />
            <div className="categoryMix" aria-label="Window insight labels">
              <span>{switchingLabel(note.switchingLevel)}</span>
              <span>{loafingLabel(note.loafingLevel)}</span>
              {compactList(note.projectHints, 2).map((hint) => (
                <span key={hint}>{shortText(hint, 56)}</span>
              ))}
            </div>
          </section>
          <TrajectoryList trajectory={note.trajectory} />
          {note.switchingEvidence || note.loafingEvidence ? (
            <ReviewDetails title="Evidence notes">
              <div className="reviewEvidence">
                {note.switchingEvidence ? <p>{note.switchingEvidence}</p> : null}
                {note.loafingEvidence ? <p>{note.loafingEvidence}</p> : null}
              </div>
            </ReviewDetails>
          ) : null}
          <RawTextDetails
            title="Raw window evidence"
            text={rawWindowEvidenceText(summary, note.rawText)}
          />
        </>
      ) : (
        <p className="redactedText insightRedacted">
          Window note generated. Text is hidden in redacted mode.
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
        <p className="reviewThesis">{observation.summaryText}</p>
      ) : (
        <p className="redactedText insightRedacted">
          Legacy screenshot note generated. Text is hidden in redacted mode.
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
      {trajectory.slice(0, 3).map((point) => (
        <li key={`${point.minuteMark}-${point.screenshotId}`}>
          <strong>Minute {point.minuteMark}</strong>
          <span>{point.observation}</span>
          <small>{categoryLabel(point.activityCategory)}</small>
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
    return <p className="emptyState">No session report yet.</p>;
  }

  const note = buildReportNote(report);

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
          <section
            className="reviewReadableSummary"
            aria-label="Session report summary"
          >
            <p className="reviewThesis">{note.mainThread}</p>
          </section>
          <ReportPhaseList phases={note.phases} />
          {note.projects.length > 0 ? (
            <div className="reviewProjectChips" aria-label="Session projects">
              <strong>Projects</strong>
              <div>
                {compactList(note.projects, 3).map((project) => (
                  <span key={project}>{shortText(project, 72)}</span>
                ))}
              </div>
            </div>
          ) : null}
          {note.fullText !== note.mainThread ? (
            <ReviewDetails title="Raw session report">
              <p>{note.fullText}</p>
            </ReviewDetails>
          ) : null}
        </>
      ) : (
        <p className="redactedText insightRedacted">
          Session report generated. Text is hidden in redacted mode.
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

function ReportPhaseList({ phases }: { phases: ReportPhase[] }) {
  if (phases.length === 0) {
    return null;
  }

  return (
    <ol className="reportPhases" aria-label="Session report phases">
      {phases.slice(0, 3).map((phase, index) => (
        <li key={`${phase.label}-${index}`}>
          <strong>{phase.label}</strong>
          <span>{phase.detail}</span>
        </li>
      ))}
    </ol>
  );
}

function ReviewFieldGrid({
  fields,
}: {
  fields: Array<[label: string, value: string]>;
}) {
  const visibleFields = fields.filter(([, value]) => value.trim().length > 0);
  if (visibleFields.length === 0) {
    return null;
  }

  return (
    <div className="reviewFieldGrid">
      {visibleFields.map(([label, value]) => (
        <div className="reviewField" key={label}>
          <strong>{label}</strong>
          <span>{value}</span>
        </div>
      ))}
    </div>
  );
}

function ReviewDetails({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <details className="reviewDetails">
      <summary role="button">{title}</summary>
      {children}
    </details>
  );
}

function RawTextDetails({ title, text }: { title: string; text?: string }) {
  if (!text) {
    return null;
  }

  return (
    <ReviewDetails title={title}>
      <pre>{text}</pre>
    </ReviewDetails>
  );
}

function rawWindowEvidenceText(
  summary: VisualWindowSummary,
  rawText?: string,
): string | undefined {
  if (summary.rawSummaryJson !== null && summary.rawSummaryJson !== undefined) {
    return JSON.stringify(summary.rawSummaryJson, null, 2);
  }

  return rawText;
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
      return "Low switching";
    case "medium":
      return "Medium switching";
    case "high":
      return "High switching";
    default:
      return "Switching unknown";
  }
}

function loafingLabel(level: string): string {
  switch (level) {
    case "none":
      return "No loafing";
    case "possible":
      return "Possible drift";
    case "clear":
      return "Clear drift";
    default:
      return "Drift unknown";
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

function compactList(values: string[], limit: number): string[] {
  if (values.length <= limit) {
    return values;
  }
  return [...values.slice(0, limit), `+${values.length - limit}`];
}

function shortText(value: string, maxLength: number): string {
  if (value.length <= maxLength) {
    return value;
  }
  return `${value.slice(0, maxLength - 1).trim()}…`;
}
